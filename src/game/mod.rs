use wgpu::*;
use block_mesh::ndshape::{ConstShape};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, RIGHT_HANDED_Y_UP_CONFIG};

pub mod camera;
use camera::CameraData;

mod lib;
pub use lib::types::*;
pub use lib::util;
pub mod texture;
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

pub struct ChunkRender {
    /// Shaders, general draw config, specs for the vertex buffers, etc.
    pipeline: RenderPipeline,
    /// The chunk mesh
    pub chunk_gpu_meshes: terrain::PosHash<GPUMesh<Vertex>>,
    stage_buffer: GreedyQuadsBuffer
}
impl ChunkRender {
    pub fn new(
        ctx: &WgpuCtx,
        camera: &CameraData,
        voxels: usize
    ) -> Self {
        // Pipeline specs for uniforms
        let layout = ctx.device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera.bind_group_layout(&ctx.device)],
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
        
        Self {
            pipeline,
            chunk_gpu_meshes: terrain::PosHash::new(),
            stage_buffer: GreedyQuadsBuffer::new(voxels)
        }
    }
    
    pub fn cache_chunk_mesh<B: MergeVoxel, SH: ConstShape<u32, 3>>(
        &mut self, 
        ctx: &WgpuCtx,
        pos: terrain::ChunkPos,
        shape: &SH,
        data: &[B], 
        size: u32,
        voxel_size: f32
    ) {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

        greedy_quads(
            data,
            shape,
            [0,0,0], // start (include padding)
            [size+1,size+1,size+1], // end (include padding)
            &faces,
            &mut self.stage_buffer // Don't allocate new memory - just pass the same mutable buffer each time.
        );

        let mut mesh = CPUMesh {
            verts: vec![],
            indxs: vec![]
        };
        // For each face of a cube
        for i in 0..6 {
            // OrientedBlockFace are used to make a quad(rectangle) face a certain way.
            let face = faces[i];
            let quads = &self.stage_buffer.quads.groups[i];

            for quad in quads.iter() {
                // Get data
                let verts = face.quad_mesh_positions(quad, voxel_size);
                let indxs = face.quad_mesh_indices(mesh.verts.len() as u32);

                // Convert to the right format
                for vert in verts {
                    mesh.verts.push(Vertex { pos: [
                        vert[0] + pos[0] as f32 * size as f32,
                        vert[1] + pos[1] as f32 * size as f32,
                        vert[2] + pos[2] as f32 * size as f32
                    ] });
                }
                for indx in indxs {
                    mesh.indxs.push(indx as Index);
                }
            }
        }

        const MAX_FACES: u32 = 16 * 16 * 16 * 6 / 2;
        const MAX_VERTS: u32 = MAX_FACES * 4; // four points
        const MAX_INDXS: u32 = MAX_FACES * 6; // two triangles(3) from the points

        self.chunk_gpu_meshes.insert(
            pos,
            mesh.upload_sized(&ctx.device, MAX_VERTS, MAX_INDXS)
        );
    }

    pub fn render<'c>(&self, ctx: &WgpuCtx, depth_texture: &Texture, camera_group: &BindGroup, chunks: &[terrain::ChunkPos]) -> Result<(), SurfaceError> {
        // Get textures to render to
        let output = ctx.surface.get_current_texture()?;
        let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());
        
        // Encodes render passes
        let mut encoder = ctx.device
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
                    view: &depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            
            pass.set_bind_group(0, camera_group, &[]);
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

        ctx.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        
        Ok(())
    }
}