//! Offline 2D FFT of the rendered sim texture, colored with a turbo map.
//!
//! Stockham DIT radix-2, two passes (rows then columns) of log2(N) butterfly
//! dispatches each, ping-ponging between two RGBA32Float storage textures.

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const FFT_N: u32 = 512;
pub const LOG2_N: u32 = 9;
const TOTAL_STAGES: u32 = 1 + 2 * LOG2_N; // init + 2*log2(N) butterflies
const PARAMS_STRIDE: u64 = 256;

pub const FFT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
pub const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    n: u32,
    stage: u32,
    axis: u32,
    _pad: u32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DisplayU {
    n: u32,
    _pad0: [u32; 3],
    _pad1: [u32; 4],
}

pub struct FftRenderer {
    // Ping-pong storage textures.
    view_a: wgpu::TextureView,
    #[allow(dead_code)]
    view_b: wgpu::TextureView,

    // Display output (RGBA8, sampled by main blit path).
    display_view: wgpu::TextureView,

    // Compute side.
    init_pipeline: wgpu::ComputePipeline,
    fft_pipeline: wgpu::ComputePipeline,
    compute_bgl: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,

    bg_sim_to_a: Option<wgpu::BindGroup>, // rebuilt when sim_view changes
    bg_a_to_b: wgpu::BindGroup,
    bg_b_to_a: wgpu::BindGroup,

    // Display side.
    display_pipeline: wgpu::RenderPipeline,
    display_bg: wgpu::BindGroup,
    display_uniform: wgpu::Buffer,
}

impl FftRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("fft-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/fft.wgsl").into()),
        });

        let make_storage_tex = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: FFT_N,
                    height: FFT_N,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FFT_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let tex_a = make_storage_tex("fft-tex-a");
        let tex_b = make_storage_tex("fft-tex-b");
        let view_a = tex_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex_b.create_view(&wgpu::TextureViewDescriptor::default());

        let display_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fft-display"),
            size: wgpu::Extent3d {
                width: FFT_N,
                height: FFT_N,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DISPLAY_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let display_view = display_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fft-params"),
            size: PARAMS_STRIDE * TOTAL_STAGES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Populate all stage parameters once; they are constant for a given FFT_N.
        let mut staging = vec![0u8; (PARAMS_STRIDE * TOTAL_STAGES as u64) as usize];
        for i in 0..TOTAL_STAGES {
            let (stage, axis) = if i == 0 {
                (0u32, 0u32) // init
            } else {
                let idx = i - 1;
                let axis = if idx < LOG2_N { 0 } else { 1 };
                let stage = idx % LOG2_N;
                (stage, axis)
            };
            let params = Params { n: FFT_N, stage, axis, _pad: 0 };
            let off = (i as u64 * PARAMS_STRIDE) as usize;
            let bytes = bytemuck::bytes_of(&params);
            staging[off..off + bytes.len()].copy_from_slice(bytes);
        }
        queue.write_buffer(&params_buffer, 0, &staging);

        let compute_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fft-compute-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: true,
                        min_binding_size: std::num::NonZeroU64::new(
                            std::mem::size_of::<Params>() as u64,
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: FFT_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
            ],
        });

        let compute_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fft-compute-layout"),
            bind_group_layouts: &[&compute_bgl],
            push_constant_ranges: &[],
        });

        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fft-init-pipeline"),
            layout: Some(&compute_layout),
            module: &shader,
            entry_point: "cs_init",
            compilation_options: Default::default(),
            cache: None,
        });
        let fft_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("fft-pipeline"),
            layout: Some(&compute_layout),
            module: &shader,
            entry_point: "cs_fft",
            compilation_options: Default::default(),
            cache: None,
        });

        let params_binding = wgpu::BufferBinding {
            buffer: &params_buffer,
            offset: 0,
            size: std::num::NonZeroU64::new(std::mem::size_of::<Params>() as u64),
        };

        let bg_a_to_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fft-bg-a-to-b"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params_binding.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
            ],
        });
        let bg_b_to_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fft-bg-b-to-a"),
            layout: &compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params_binding.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_b),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
            ],
        });

        // Display pipeline.
        let display_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("fft-display-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });

        let display_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("fft-display-uniform"),
            contents: bytemuck::bytes_of(&DisplayU {
                n: FFT_N,
                _pad0: [0; 3],
                _pad1: [0; 4],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let display_bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fft-display-bg"),
            layout: &display_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: display_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view_a),
                },
            ],
        });

        let display_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("fft-display-layout"),
            bind_group_layouts: &[&display_bgl],
            push_constant_ranges: &[],
        });

        let display_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("fft-display-pipeline"),
                layout: Some(&display_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: "vs_display",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: "fs_display",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: DISPLAY_FORMAT,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        Self {
            view_a,
            view_b,
            display_view,
            init_pipeline,
            fft_pipeline,
            compute_bgl,
            params_buffer,
            bg_sim_to_a: None,
            bg_a_to_b,
            bg_b_to_a,
            display_pipeline,
            display_bg,
            display_uniform,
        }
    }

    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.display_view
    }

    /// Must be called whenever the sim texture is (re)created so the init
    /// bind group points at the current texture view.
    pub fn update_sim_view(&mut self, device: &wgpu::Device, sim_view: &wgpu::TextureView) {
        let params_binding = wgpu::BufferBinding {
            buffer: &self.params_buffer,
            offset: 0,
            size: std::num::NonZeroU64::new(std::mem::size_of::<Params>() as u64),
        };
        self.bg_sim_to_a = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fft-bg-sim-to-a"),
            layout: &self.compute_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(params_binding),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(sim_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.view_a),
                },
            ],
        }));
    }

    pub fn run(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(bg_sim) = self.bg_sim_to_a.as_ref() else {
            return;
        };

        let workgroups_xy = (FFT_N + 7) / 8;
        let workgroups_half = ((FFT_N >> 1) + 7) / 8;

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("fft-init-pass"),
                timestamp_writes: None,
            });
            cpass.set_pipeline(&self.init_pipeline);
            cpass.set_bind_group(0, bg_sim, &[0]);
            cpass.dispatch_workgroups(workgroups_xy, workgroups_xy, 1);
        }

        // Butterfly passes. After init, live data is in view_a.
        // axis 0: 9 stages starting from A→B.
        // axis 1: 9 stages starting from B→A (since axis-0 leaves data in B).
        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("fft-butterfly-pass"),
            timestamp_writes: None,
        });
        cpass.set_pipeline(&self.fft_pipeline);
        for stage_idx in 0..TOTAL_STAGES - 1 {
            let offset = ((stage_idx + 1) as u64 * PARAMS_STRIDE) as u32;
            // Parity determines A↔B direction.
            // stage_idx=0 (first fft pass) → A→B; then alternate.
            let a_to_b = stage_idx % 2 == 0;
            let bg = if a_to_b { &self.bg_a_to_b } else { &self.bg_b_to_a };
            cpass.set_bind_group(0, bg, &[offset]);
            cpass.dispatch_workgroups(workgroups_half, workgroups_xy, 1);
        }
    }

    pub fn draw_display(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("fft-display-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.display_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rpass.set_pipeline(&self.display_pipeline);
        rpass.set_bind_group(0, &self.display_bg, &[]);
        rpass.draw(0..3, 0..1);
        let _ = &self.display_uniform;
    }
}
