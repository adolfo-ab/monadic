use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const MAX_EMITTERS: u64 = 1024;
pub const MAX_SPEC: u64 = 16;

const SIM_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Uniforms {
    pub resolution: [f32; 2],
    pub canvas_origin: [f32; 2],
    pub canvas_size: [f32; 2],
    pub time: f32,
    pub num_emitters: u32,
    pub wave_speed: f32,
    pub amp_scale: f32,
    pub color_mode: u32,
    pub decay_mode: u32,
    pub num_spec: u32,
    pub phase_mode: u32,
    pub phase_param_a: f32,
    pub phase_param_b: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct BlitUniforms {
    pub dst_origin: [f32; 2],
    pub dst_size: [f32; 2],
    pub screen_size: [f32; 2],
    pub _pad: [f32; 2],
}

pub struct WaveRenderer {
    wave_pipeline: wgpu::RenderPipeline,
    wave_bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    emitter_buffer: wgpu::Buffer,
    spectrum_buffer: wgpu::Buffer,

    sim_size: u32,
    sim_view: wgpu::TextureView,

    blit_pipeline: wgpu::RenderPipeline,
    blit_bgl: wgpu::BindGroupLayout,
    blit_bind_group: wgpu::BindGroup,
    blit_uniform_buffer: wgpu::Buffer,
    blit_sampler: wgpu::Sampler,
}

impl WaveRenderer {
    pub fn new(device: &wgpu::Device, surface_format: wgpu::TextureFormat, sim_size: u32) -> Self {
        let wave_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wave-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/wave.wgsl").into()),
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("blit-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/blit.wgsl").into()),
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("wave-uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let emitter_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wave-emitters"),
            contents: bytemuck::cast_slice(&[[0.0f32; 4]; MAX_EMITTERS as usize]),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let mut init_spec = [[0.0f32; 4]; MAX_SPEC as usize];
        init_spec[0] = [1.0, 1.0, 0.0, 0.0];
        let spectrum_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wave-spectrum"),
            contents: bytemuck::cast_slice(&init_spec),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let wave_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("wave-bgl"),
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let wave_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wave-bg"),
            layout: &wave_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: emitter_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: spectrum_buffer.as_entire_binding(),
                },
            ],
        });

        let wave_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wave-pipeline-layout"),
            bind_group_layouts: &[&wave_bgl],
            push_constant_ranges: &[],
        });

        let wave_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wave-pipeline"),
            layout: Some(&wave_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &wave_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &wave_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: SIM_FORMAT,
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

        // Blit resources.
        let blit_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("blit-uniforms"),
            size: std::mem::size_of::<BlitUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("blit-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let blit_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("blit-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
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
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("blit-pipeline-layout"),
            bind_group_layouts: &[&blit_bgl],
            push_constant_ranges: &[],
        });

        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("blit-pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
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

        let sim_view = create_sim_view(device, sim_size);
        let blit_bind_group = create_blit_bind_group(
            device,
            &blit_bgl,
            &blit_uniform_buffer,
            &sim_view,
            &blit_sampler,
        );

        Self {
            wave_pipeline,
            wave_bind_group,
            uniform_buffer,
            emitter_buffer,
            spectrum_buffer,
            sim_size,
            sim_view,
            blit_pipeline,
            blit_bgl,
            blit_bind_group,
            blit_uniform_buffer,
            blit_sampler,
        }
    }

    pub fn ensure_sim_size(&mut self, device: &wgpu::Device, size: u32) {
        let size = size.max(1);
        if size == self.sim_size {
            return;
        }
        self.sim_size = size;
        self.sim_view = create_sim_view(device, size);
        self.blit_bind_group = create_blit_bind_group(
            device,
            &self.blit_bgl,
            &self.blit_uniform_buffer,
            &self.sim_view,
            &self.blit_sampler,
        );
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    pub fn update_blit_uniforms(&self, queue: &wgpu::Queue, uniforms: &BlitUniforms) {
        queue.write_buffer(&self.blit_uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    pub fn update_emitters(&self, queue: &wgpu::Queue, emitters: &[[f32; 4]]) {
        let n = (emitters.len() as u64).min(MAX_EMITTERS) as usize;
        if n == 0 {
            return;
        }
        queue.write_buffer(&self.emitter_buffer, 0, bytemuck::cast_slice(&emitters[..n]));
    }

    pub fn update_spectrum(&self, queue: &wgpu::Queue, spec: &[[f32; 4]]) {
        let n = (spec.len() as u64).min(MAX_SPEC) as usize;
        if n == 0 {
            return;
        }
        queue.write_buffer(&self.spectrum_buffer, 0, bytemuck::cast_slice(&spec[..n]));
    }

    pub fn render_sim(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sim-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.sim_view,
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
        rpass.set_pipeline(&self.wave_pipeline);
        rpass.set_bind_group(0, &self.wave_bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }

    pub fn draw_blit<'rp>(&'rp self, rpass: &mut wgpu::RenderPass<'rp>) {
        rpass.set_pipeline(&self.blit_pipeline);
        rpass.set_bind_group(0, &self.blit_bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}

fn create_sim_view(device: &wgpu::Device, size: u32) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sim-texture"),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: SIM_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    tex.create_view(&wgpu::TextureViewDescriptor::default())
}

fn create_blit_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    view: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit-bg"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
        ],
    })
}
