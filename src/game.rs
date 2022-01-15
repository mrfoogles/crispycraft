#![allow(dead_code)]
use wgpu::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

use winit::window::Window;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy)]
pub struct Vertex {
    pos: [f32; 3]
}
impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[
            VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            },
            VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as BufferAddress,
                shader_location: 1,
                format: VertexFormat::Float32x3,
            }
            ]
        }
    }
}

pub type Index = u16;
const INDEX_FORMAT: IndexFormat = IndexFormat::Uint16;
const VERT_ENTRY_POINT: &str = "vs_main";
const FRAG_ENTRY_POINT: &str = "fs_main";

pub struct CPUMesh {
    verts: Vec<Vertex>,
    indxs: Vec<Index>
}
impl CPUMesh {
    fn upload(&self, device: &Device) -> GPUMesh {
        let vertbuf = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.verts),
                usage: BufferUsages::VERTEX | BufferUsages::COPY_DST,
            }
        );
        let indxbuf = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&self.indxs),
                usage: BufferUsages::INDEX | BufferUsages::COPY_DST,
            }
        );

        return GPUMesh {
            vertbuf,
            num_verts: self.verts.len() as u32,
            indxbuf,
            num_indxs: self.indxs.len() as u32
        }
    }
    fn update_gpu_mesh(&self, gpu_mesh: &mut GPUMesh, queue: &Queue) {
        queue.write_buffer(&gpu_mesh.vertbuf, 0, bytemuck::cast_slice(&self.verts));
        queue.write_buffer(&gpu_mesh.indxbuf, 0, bytemuck::cast_slice(&self.indxs));
    }
}

pub struct GPUMesh {
    vertbuf: Buffer,
    num_verts: u32,
    indxbuf: Buffer,
    num_indxs: u32
}


fn draw_mesh<'a,'b: 'a>(pass: &mut RenderPass<'a>, mesh: &'b GPUMesh, instances: u32) {
    pass.set_vertex_buffer(0,mesh.vertbuf.slice(..));
    pass.set_index_buffer(mesh.indxbuf.slice(..), INDEX_FORMAT);

    pass.draw_indexed(0..mesh.num_indxs, 0, 0..instances);
}

pub struct State {
    device: Device,
    queue: Queue,
    surface: Surface,
    config: SurfaceConfiguration,
    pub size: winit::dpi::PhysicalSize<u32>,

    pipeline: RenderPipeline,
    gpu_mesh: GPUMesh
}
impl State {
    pub async fn new(window: &Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(include_str!("./shader.wgsl").into()),
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: VERT_ENTRY_POINT,
                buffers: &[Vertex::desc()]
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: &FRAG_ENTRY_POINT,
                targets: &[ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL
                }]
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,//Some(Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None
        });

        let gpu_mesh = CPUMesh {
            verts: vec![
                Vertex {
                    pos: [0.0,0.5,0.0]
                },
                Vertex {
                    pos: [-0.5,-0.5,0.0]
                },
                Vertex {
                    pos: [0.5,-0.5,0.0]
                },
            ],
            indxs: vec![0,1,2]
        }.upload(&device);

        Self {
            surface,
            device,
            queue,
            config,
            size,

            pipeline,
            gpu_mesh
        }
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn render(&self) -> Result<(), SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&CommandEncoderDescriptor {
            label: None
        });
        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: None,
                color_attachments: &[RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None // TODO!!!
            });

            pass.set_pipeline(&self.pipeline);
            draw_mesh(&mut pass, &self.gpu_mesh, 1);
        };

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        return Ok(());
    }
}