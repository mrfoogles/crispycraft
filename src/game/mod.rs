#![allow(dead_code)]
use wgpu::*;
use wgpu::util::{DeviceExt};

use winit::window::Window;

pub mod camera;
mod texture;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3]
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
            }
            ]
        }
    }
}

pub type Index = u16;
const INDEX_FORMAT: IndexFormat = IndexFormat::Uint16;
const VERT_ENTRY_POINT: &str = "vs_main";
const FRAG_ENTRY_POINT: &str = "fs_main";

fn fast_buffer<T: bytemuck::Pod>(device: &Device, data: &[T], usage: BufferUsages) -> Buffer {
    return device.create_buffer_init(
        &wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Buffer", usage)),
            contents: bytemuck::cast_slice(data),
            usage,
        }
    );
}

#[derive(Debug)]
pub struct CPUMesh {
    pub verts: Vec<Vertex>,
    pub indxs: Vec<Index>
}
impl CPUMesh {
    fn upload(&self, device: &Device) -> GPUMesh {
        let vertbuf = fast_buffer(device, &self.verts, BufferUsages::VERTEX | BufferUsages::COPY_DST);
        let indxbuf = fast_buffer(device, &self.indxs, BufferUsages::INDEX | BufferUsages::COPY_DST);

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
    gpu_mesh: GPUMesh,

    camera_buffer: Buffer,
    camera_group: BindGroup,

    depth_texture: texture::Texture
}
impl State {
    pub async fn new(window: &Window, mesh: &CPUMesh, camera: &camera::CameraData) -> Self {
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

        let camera_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                // Camera buffer
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None
                    },
                    count: None
                }
            ]
        });
        let camera_buffer = fast_buffer(&device, &[camera.uniform()], BufferUsages::COPY_DST | BufferUsages::UNIFORM);
        let camera_group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout: &camera_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding()
                }
            ]
        });

        let layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[]
        });

        let shader_text = std::fs::read_to_string(
            format!("{}/src/shader.wgsl", env!("CARGO_MANIFEST_DIR"))
        ).unwrap();
        let shader = device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl(
                (&shader_text).into()
            ),
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
                entry_point: FRAG_ENTRY_POINT,
                targets: &[ColorTargetState {
                    format: config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL
                }]
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None
        });

        let gpu_mesh = mesh.upload(&device);
        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        Self {
            surface,
            device,
            queue,
            config,
            size,

            pipeline,
            gpu_mesh,

            camera_buffer,
            camera_group,

            depth_texture
        }
    }

    pub fn update_camera(&mut self, camera: &camera::CameraData) {
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[camera.uniform()]));
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
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
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            pass.set_bind_group(0,&self.camera_group, &[]);
            pass.set_pipeline(&self.pipeline);
            draw_mesh(&mut pass, &self.gpu_mesh, 1);
        };

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        return Ok(());
    }
}