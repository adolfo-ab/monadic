use std::sync::Arc;
use std::time::Instant;

use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::fft::FftRenderer;
use crate::renderer::{BlitUniforms, Uniforms, WaveRenderer};
use crate::state::{ColorMode, SimState};
use crate::ui;

pub struct App {
    state: Option<RuntimeState>,
    sim: SimState,
    last_frame: Instant,
}

#[derive(Copy, Clone, PartialEq, Eq)]
enum BlitSource {
    Sim,
    Fft,
}

impl App {
    pub fn new() -> Self {
        Self {
            state: None,
            sim: SimState::default(),
            last_frame: Instant::now(),
        }
    }
}

struct RuntimeState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: WaveRenderer,
    fft: FftRenderer,
    blit_mode: BlitSource,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() {
            return;
        }

        let attrs = Window::default_attributes()
            .with_title("monadic")
            .with_inner_size(winit::dpi::PhysicalSize::new(
                (ui::PANEL_WIDTH as u32) + 1024,
                1024u32,
            ));
        let window = Arc::new(event_loop.create_window(attrs).expect("create window"));

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("create surface");

        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .expect("no compatible GPU adapter");

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::default(),
            },
            None,
        ))
        .expect("request device");

        let size = window.inner_size();
        let caps = surface.get_capabilities(&adapter);
        // Prefer a non-sRGB format so egui's sRGB-aware shader doesn't double-correct.
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let renderer = WaveRenderer::new(&device, format, self.sim.sim_resolution);
        let mut fft = FftRenderer::new(&device, &queue);
        fft.update_sim_view(&device, renderer.sim_view());

        let egui_ctx = egui::Context::default();
        ui::install_fonts(&egui_ctx);
        ui::install_style(&egui_ctx);
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, format, None, 1, false);

        // Push initial emitter + spectrum buffers.
        let emitters = self.sim.build_emitters();
        renderer.update_emitters(&queue, &emitters);
        self.sim.emitters_dirty = false;
        let spec = self.sim.build_spectrum();
        renderer.update_spectrum(&queue, &spec);
        self.sim.spectrum_dirty = false;

        self.state = Some(RuntimeState {
            window,
            surface,
            device,
            queue,
            config,
            renderer,
            fft,
            blit_mode: BlitSource::Sim,
            egui_ctx,
            egui_state,
            egui_renderer,
        });
        self.last_frame = Instant::now();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        let response = state
            .egui_state
            .on_window_event(state.window.as_ref(), &event);
        if response.repaint {
            state.window.request_redraw();
        }
        if response.consumed {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    state.config.width = size.width;
                    state.config.height = size.height;
                    state.surface.configure(&state.device, &state.config);
                    state.window.request_redraw();
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = self.state.as_ref() {
            state.window.request_redraw();
        }
    }
}

impl App {
    fn render(&mut self) {
        let Some(state) = self.state.as_mut() else {
            return;
        };

        // Tick clock.
        let now = Instant::now();
        let dt = (now - self.last_frame).as_secs_f32().min(0.1);
        self.last_frame = now;
        if !self.sim.paused {
            self.sim.time += dt;
        }

        // Compute canvas rectangle = window area minus left UI panel.
        let pixels_per_point = state.window.scale_factor() as f32;
        let panel_px = ui::PANEL_WIDTH * pixels_per_point;

        let canvas_w = (state.config.width as f32 - panel_px).max(1.0);
        let canvas_h = state.config.height as f32;
        let canvas_size_px = canvas_w.min(canvas_h);
        // Center canvas in the available area.
        let canvas_origin = [
            panel_px + (canvas_w - canvas_size_px) * 0.5,
            (canvas_h - canvas_size_px) * 0.5,
        ];

        // Resize offscreen sim texture if requested resolution changed.
        let sim_resized = state
            .renderer
            .ensure_sim_size(&state.device, self.sim.sim_resolution);
        if sim_resized {
            state.fft.update_sim_view(&state.device, state.renderer.sim_view());
            // Rebind whichever source is currently active to the new view.
            match state.blit_mode {
                BlitSource::Sim => state.renderer.restore_blit_source(&state.device),
                BlitSource::Fft => {
                    state
                        .renderer
                        .set_blit_source(&state.device, state.fft.display_view());
                }
            }
        }

        if self.sim.emitters_dirty {
            let emitters = self.sim.build_emitters();
            state.renderer.update_emitters(&state.queue, &emitters);
            self.sim.emitters_dirty = false;
        }
        let spec = self.sim.build_spectrum();
        let spec_count = spec.len() as u32;
        if self.sim.spectrum_dirty {
            state.renderer.update_spectrum(&state.queue, &spec);
            self.sim.spectrum_dirty = false;
        }

        let sim_px = self.sim.sim_resolution as f32;
        let uniforms = Uniforms {
            resolution: [sim_px, sim_px],
            canvas_origin: [0.0, 0.0],
            canvas_size: [sim_px, sim_px],
            time: self.sim.time,
            num_emitters: self.sim.num_nodes as u32,
            wave_speed: self.sim.wave_speed,
            amp_scale: self.sim.amp_scale,
            color_mode: self.sim.color_mode_u32(),
            decay_mode: self.sim.decay_mode_u32(),
            num_spec: spec_count,
            phase_mode: self.sim.phase_mode.id(),
            phase_param_a: self.sim.phase_param_a,
            phase_param_b: self.sim.phase_param_b,
            wave_shape: self.sim.wave_shape.id(),
            shape_param_a: self.sim.shape_param_a,
            shape_param_b: self.sim.shape_param_b,
            _pad: 0.0,
            spec_motion: self.sim.spec_motion.id(),
            spec_motion_rate: self.sim.spec_motion_rate,
            spec_motion_depth: self.sim.spec_motion_depth,
            _pad2: 0.0,
        };
        state.renderer.update_uniforms(&state.queue, &uniforms);

        let blit_uniforms = BlitUniforms {
            dst_origin: canvas_origin,
            dst_size: [canvas_size_px, canvas_size_px],
            screen_size: [state.config.width as f32, state.config.height as f32],
            _pad: [0.0, 0.0],
        };
        state
            .renderer
            .update_blit_uniforms(&state.queue, &blit_uniforms);

        // Run egui.
        let raw_input = state.egui_state.take_egui_input(state.window.as_ref());
        let full_output = state.egui_ctx.run(raw_input, |ctx| {
            ui::draw(ctx, &mut self.sim);
        });
        state
            .egui_state
            .handle_platform_output(state.window.as_ref(), full_output.platform_output);
        let tris = state
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, image_delta) in &full_output.textures_delta.set {
            state
                .egui_renderer
                .update_texture(&state.device, &state.queue, *id, image_delta);
        }

        // Acquire frame.
        let frame = match state.surface.get_current_texture() {
            Ok(f) => f,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                state.surface.configure(&state.device, &state.config);
                return;
            }
            Err(e) => {
                log::error!("surface error: {e:?}");
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = state
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame-encoder"),
            });

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [state.config.width, state.config.height],
            pixels_per_point,
        };
        state.egui_renderer.update_buffers(
            &state.device,
            &state.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        // Offscreen sim pass.
        state.renderer.render_sim(&mut encoder);

        // Ensure the blit samples the texture matching the current view mode,
        // and run the FFT pipeline when active.
        let want_fft = self.sim.color_mode == ColorMode::Fft;
        let want_mode = if want_fft { BlitSource::Fft } else { BlitSource::Sim };
        if want_mode != state.blit_mode {
            match want_mode {
                BlitSource::Sim => state.renderer.restore_blit_source(&state.device),
                BlitSource::Fft => {
                    state
                        .renderer
                        .set_blit_source(&state.device, state.fft.display_view());
                }
            }
            state.blit_mode = want_mode;
        }
        if want_fft {
            state.fft.run(&mut encoder);
            state.fft.draw_display(&mut encoder);
        }

        // Blit sim texture into canvas rect on the surface.
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("blit-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            state.renderer.draw_blit(&mut rpass);
        }

        // egui pass.
        {
            let mut rpass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui-pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            state
                .egui_renderer
                .render(&mut rpass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            state.egui_renderer.free_texture(id);
        }

        state.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }
}
