//! Gray-Scott reaction-diffusion coupled to the wave sim texture.
//!
//! Two ping-pong Rgba32Float state textures hold (U, V). Each frame:
//!   1. optional reseed → cs_init writes into `state_a` (front = A).
//!   2. `substeps` × cs_step: read `front`, write `back`, swap.
//!   3. cs_display colorizes `front` into `display_view` for blit.

use bytemuck::{Pod, Zeroable};

pub const RD_N: u32 = 256;
const STATE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
const DISPLAY_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Params {
    pub n: u32,
    pub num_emitters: u32,
    pub emit_radius: f32,
    pub emit_rate: f32,
    pub feed: f32,
    pub kill: f32,
    pub coupling: f32,
    pub dt: f32,
    pub diff_u: f32,
    pub diff_v: f32,
    pub time: f32,
    pub _pad2: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DispU {
    n: u32,
    _p0: u32,
    _p1: u32,
    _p2: u32,
}

pub struct RdRenderer {
    view_a: wgpu::TextureView,
    view_b: wgpu::TextureView,
    display_view: wgpu::TextureView,

    step_pipeline: wgpu::ComputePipeline,
    init_pipeline: wgpu::ComputePipeline,
    step_bgl: wgpu::BindGroupLayout,
    params_buffer: wgpu::Buffer,

    // step_ab reads A (src tex), writes B (dst storage)
    bg_step_ab: Option<wgpu::BindGroup>,
    bg_step_ba: Option<wgpu::BindGroup>,
    // init writes into A (or B) — built alongside step groups when bindings are supplied.
    bg_init_a: Option<wgpu::BindGroup>,
    bg_init_b: Option<wgpu::BindGroup>,

    display_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    display_uniform: wgpu::Buffer,
    display_bg_from_a: wgpu::BindGroup,
    display_bg_from_b: wgpu::BindGroup,

    front_is_a: bool,
    initialized: bool,
}

impl RdRenderer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("rd-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/rd.wgsl").into()),
        });

        let make_state = |label: &str| {
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some(label),
                size: wgpu::Extent3d {
                    width: RD_N,
                    height: RD_N,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: STATE_FORMAT,
                usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            })
        };
        let tex_a = make_state("rd-state-a");
        let tex_b = make_state("rd-state-b");
        let view_a = tex_a.create_view(&wgpu::TextureViewDescriptor::default());
        let view_b = tex_b.create_view(&wgpu::TextureViewDescriptor::default());

        let display_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("rd-display"),
            size: wgpu::Extent3d {
                width: RD_N,
                height: RD_N,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DISPLAY_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let display_view = display_tex.create_view(&wgpu::TextureViewDescriptor::default());

        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rd-params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let step_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rd-step-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
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
                        format: STATE_FORMAT,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let step_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rd-step-layout"),
            bind_group_layouts: &[&step_bgl],
            push_constant_ranges: &[],
        });

        let step_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rd-step-pipeline"),
            layout: Some(&step_layout),
            module: &shader,
            entry_point: "cs_step",
            compilation_options: Default::default(),
            cache: None,
        });
        let init_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("rd-init-pipeline"),
            layout: Some(&step_layout),
            module: &shader,
            entry_point: "cs_init",
            compilation_options: Default::default(),
            cache: None,
        });

        // bg_init_a / bg_init_b / step groups are built in `update_bindings` once
        // sim_view and emitter buffer are known.

        // Display side.
        let display_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rd-display-bgl"),
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

        let display_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rd-display-uniform"),
            size: std::mem::size_of::<DispU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(
            &display_uniform,
            0,
            bytemuck::bytes_of(&DispU { n: RD_N, _p0: 0, _p1: 0, _p2: 0 }),
        );

        let display_bg_from_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-display-from-a"),
            layout: &display_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: display_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_a) },
            ],
        });
        let display_bg_from_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-display-from-b"),
            layout: &display_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: display_uniform.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view_b) },
            ],
        });

        let display_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("rd-display-layout"),
            bind_group_layouts: &[&display_bgl],
            push_constant_ranges: &[],
        });

        let display_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("rd-display-pipeline"),
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
            step_pipeline,
            init_pipeline,
            step_bgl,
            params_buffer,
            bg_step_ab: None,
            bg_step_ba: None,
            bg_init_a: None,
            bg_init_b: None,
            display_pipeline,
            display_uniform,
            display_bg_from_a,
            display_bg_from_b,
            front_is_a: true,
            initialized: false,
        }
    }

    pub fn display_view(&self) -> &wgpu::TextureView {
        &self.display_view
    }

    /// Rebuild all compute bind groups. Call when sim_view or emitter buffer changes.
    pub fn update_bindings(
        &mut self,
        device: &wgpu::Device,
        sim_view: &wgpu::TextureView,
        emitter_buffer: &wgpu::Buffer,
    ) {
        self.bg_step_ab = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-step-ab"),
            layout: &self.step_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.view_b) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(sim_view) },
                wgpu::BindGroupEntry { binding: 4, resource: emitter_buffer.as_entire_binding() },
            ],
        }));
        self.bg_step_ba = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-step-ba"),
            layout: &self.step_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.view_a) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(sim_view) },
                wgpu::BindGroupEntry { binding: 4, resource: emitter_buffer.as_entire_binding() },
            ],
        }));
        self.bg_init_a = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-init-a"),
            layout: &self.step_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view_b) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.view_a) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(sim_view) },
                wgpu::BindGroupEntry { binding: 4, resource: emitter_buffer.as_entire_binding() },
            ],
        }));
        self.bg_init_b = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rd-init-b"),
            layout: &self.step_bgl,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: self.params_buffer.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self.view_a) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.view_b) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(sim_view) },
                wgpu::BindGroupEntry { binding: 4, resource: emitter_buffer.as_entire_binding() },
            ],
        }));
    }

    pub fn update_params(&self, queue: &wgpu::Queue, params: &Params) {
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(params));
    }

    pub fn request_reset(&mut self) {
        self.initialized = false;
    }

    pub fn run(&mut self, encoder: &mut wgpu::CommandEncoder, substeps: u32) {
        let (Some(bg_ab), Some(bg_ba), Some(bg_ia), Some(bg_ib)) = (
            self.bg_step_ab.as_ref(),
            self.bg_step_ba.as_ref(),
            self.bg_init_a.as_ref(),
            self.bg_init_b.as_ref(),
        ) else {
            return;
        };
        let groups = (RD_N + 7) / 8;

        let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("rd-pass"),
            timestamp_writes: None,
        });

        if !self.initialized {
            cpass.set_pipeline(&self.init_pipeline);
            cpass.set_bind_group(0, bg_ia, &[]);
            cpass.dispatch_workgroups(groups, groups, 1);
            // Also seed B so first B→A read is valid.
            cpass.set_bind_group(0, bg_ib, &[]);
            cpass.dispatch_workgroups(groups, groups, 1);
            self.initialized = true;
            self.front_is_a = true;
        }

        cpass.set_pipeline(&self.step_pipeline);
        for _ in 0..substeps {
            let bg = if self.front_is_a { bg_ab } else { bg_ba };
            cpass.set_bind_group(0, bg, &[]);
            cpass.dispatch_workgroups(groups, groups, 1);
            self.front_is_a = !self.front_is_a;
        }
    }

    pub fn draw_display(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("rd-display-pass"),
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
        let bg = if self.front_is_a {
            &self.display_bg_from_a
        } else {
            &self.display_bg_from_b
        };
        rpass.set_pipeline(&self.display_pipeline);
        rpass.set_bind_group(0, bg, &[]);
        rpass.draw(0..3, 0..1);
    }
}

