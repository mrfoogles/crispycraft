use wgpu::*;
use winit::window::Window;

pub mod camera;

mod lib;
pub use lib::types::*;
pub use lib::util;
mod texture;
use texture::Texture;

use crate::terrain;

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct Vertex {
    pub pos: [f32; 3],
}
impl Vertex {
    fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: VertexFormat::Float32x3,
            }],
        }
    }
}

impl BindGroupSource<Buffer> for camera::CameraData {
    fn bind_group_layout(&self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
            // Camera buffer
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            ],
        })
    }
    
    fn bind_group(
        &self,
        device: &Device,
        _queue: &Queue,
        layout: &BindGroupLayout,
    ) -> (BindGroup, Buffer) {
        let buffer = util::fast_buffer(
            device,
            &[self.uniform()],
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        );
        let group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        
        (group, buffer)
    }
    
    // Optional to implement
    fn update_bind_group(&self, data: &Buffer, queue: &Queue) {
        queue.write_buffer(data, 0, bytemuck::cast_slice(&[self.uniform()]));
    }
}

pub struct RenderState {
    pub ctx: WgpuCtx,
    
    /// Shaders, general draw config, specs for the vertex buffers, etc.
    pipeline: RenderPipeline,
    /// The chunk mesh
    pub chunk_gpu_meshes: terrain::PosHash<GPUMesh<Vertex>>,
    
    /// Camera data on the GPU
    pub camera_buffer: Buffer,
    /// Handle on camera data for draw calls
    camera_group: BindGroup,
    
    depth_texture: Texture,
}
impl RenderState {
    pub async fn new(
        window: &Window,
        camera: &camera::CameraData,
    ) -> Self {
        // Set up a WebGPU context
        let ctx = WgpuCtx::default(window).await;
        
        // Set up the camera GPU buffer & add the buffer to the pipeline specs
        let camera_layout = camera.bind_group_layout(&ctx.device);
        let (camera_group, camera_buffer) = camera.bind_group(&ctx.device, &ctx.queue, &camera_layout);
        
        // Pipeline specs for uniforms
        let layout = ctx.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_layout],
            push_constant_ranges: &[],
        });
        
        let shader_text =
        std::fs::read_to_string(format!("{}/src/shader.wgsl", env!("CARGO_MANIFEST_DIR")))
        .unwrap();
        let shader = ctx.device.create_shader_module(&ShaderModuleDescriptor {
            label: Some("Shader"),
            source: ShaderSource::Wgsl((&shader_text).into()),
        });
        
        let pipeline = ctx.device.create_render_pipeline(&RenderPipelineDescriptor {
            label: None,
            layout: Some(&layout),
            vertex: VertexState {
                module: &shader,
                entry_point: lib::VERT_ENTRY_POINT,
                buffers: &[Vertex::desc()],
            },
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: lib::FRAG_ENTRY_POINT,
                targets: &[ColorTargetState {
                    format: ctx.config.format,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                }],
            }),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: Some(Face::Back),
                ..PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: MultisampleState::default(),
            multiview: None,
        });
        
        let depth_texture = Texture::create_depth_texture(&ctx.device, &ctx.config, "depth_texture");
        
        Self {
            ctx,
            
            pipeline,
            chunk_gpu_meshes: terrain::PosHash::new(),
            
            camera_buffer,
            camera_group,
            
            depth_texture,
        }
    }
    
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.ctx.resize(new_size);
            
            self.depth_texture =
            texture::Texture::create_depth_texture(&self.ctx.device, &self.ctx.config, "depth_texture");
        }
    }
    
    pub fn render<'c>(&self, chunks: &[terrain::ChunkPos]) -> Result<(), SurfaceError> {
        // Get textures to render to
        let output = self.ctx.surface.get_current_texture()?;
        let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
        
        // Encodes render passes
        let mut encoder = self.ctx.device
        .create_command_encoder(&CommandEncoderDescriptor { label: None });

        // You have to drop the pass once you're done with it, so it's in a temporary scope
        {
            // A render pass is draws some vertices w/ pipeline & bind groups
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
            
            pass.set_bind_group(0, &self.camera_group, &[]);
            pass.set_pipeline(&self.pipeline);
            
            for pos in chunks {
                match self.chunk_gpu_meshes.get(pos) {
                    Some(mesh) => {
                        util::draw_mesh(&mut pass, mesh, 1);
                    },
                    None => { panic!("Mesh not set - {:?}", pos) }
                }
            }
        };

        self.ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
}