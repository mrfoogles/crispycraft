#![allow(dead_code)]
use wgpu::*;
use wgpu::util::{BufferInitDescriptor, DeviceExt};

#[repr(C)]
pub struct Vertex {
    pos: [f32; 3]
}

pub type Index = u16;
const INDEX_FORMAT: IndexFormat = IndexFormat::Uint16;

pub struct CPUMesh {
    verts: Vec<Vertex>,
    indxs: Vec<Index>
}
pub struct GPUMesh {
    vertbuf: Buffer,
    num_verts: u32,
    indxbuf: Buffer,
    num_indxs: u32
}
impl GPUMesh {
    fn from_cpu(device: &Device, cpu_mesh: &CPUMesh) -> Self {
        todo!()
    }
    fn update_from_cpu(&mut self, queue: &Queue, cpu_mesh: &CPUMesh) {
        todo!()
    }
}
// self.gpu_mesh = GPUMesh::from_cpu(mesh::mesh_from_chunk(&chunk, &mut stage_buffer))
// self.gpu_mesh.update_from_cpu(mesh::mesh_from_chunk(&quads_buffer))


fn draw_mesh<'a,'b: 'a>(pass: &mut RenderPass<'a>, mesh: &'b GPUMesh, instances: u32) {
    pass.set_vertex_buffer(0,mesh.vertbuf.slice(..));
    pass.set_index_buffer(mesh.indxbuf.slice(..), INDEX_FORMAT);

    pass.draw_indexed(0..mesh.num_indxs, 0, 0..instances);
}

struct State {
    device: Device,
    queue: Queue,
    surface: Surface,

    pipeline: RenderPipeline,
    gpu_mesh: GPUMesh
}
impl State {
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