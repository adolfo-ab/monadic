use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

pub const MAX_EMITTERS: u64 = 1024;

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
}

pub struct WaveRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    emitter_buffer: wgpu::Buffer,
}

impl WaveRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wave-shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/wave.wgsl").into()),
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

        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("wave-bg"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: emitter_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wave-pipeline-layout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wave-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
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
            pipeline,
            bind_group,
            uniform_buffer,
            emitter_buffer,
        }
    }

    pub fn update_uniforms(&self, queue: &wgpu::Queue, uniforms: &Uniforms) {
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(uniforms));
    }

    pub fn update_emitters(&self, queue: &wgpu::Queue, emitters: &[[f32; 4]]) {
        let n = (emitters.len() as u64).min(MAX_EMITTERS) as usize;
        if n == 0 {
            return;
        }
        queue.write_buffer(&self.emitter_buffer, 0, bytemuck::cast_slice(&emitters[..n]));
    }

    pub fn draw<'rp>(&'rp self, rpass: &mut wgpu::RenderPass<'rp>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_bind_group(0, &self.bind_group, &[]);
        rpass.draw(0..3, 0..1);
    }
}
