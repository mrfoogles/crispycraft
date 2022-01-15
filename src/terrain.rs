use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, RIGHT_HANDED_Y_UP_CONFIG};

use crate::game::{Vertex, Index, CPUMesh};

#[derive(Copy, Clone, Eq, PartialEq)]
struct Block {
    solid: bool
}

impl Voxel for Block {
    fn is_empty(&self) -> bool {
        ! self.solid
    }

    fn is_opaque(&self) -> bool {
        self.solid
    }
}

impl MergeVoxel for Block {
    type MergeValue = bool;

    fn merge_value(&self) -> Self::MergeValue {
        self.solid
    }
}

// 16x16x16 with 1-block padding on edges
type ChunkShape = ConstShape3u32<18, 18, 18>;

pub struct TerrainState {
    chunk: [Block; ChunkShape::SIZE as usize],
    stage_buffer: GreedyQuadsBuffer
}
impl TerrainState {
    pub fn new() -> Self {
        let mut chunk = [Block { solid: false }; ChunkShape::SIZE as usize];
        for i in 0..ChunkShape::SIZE {
            let [x, y, z] = ChunkShape::delinearize(i);
            chunk[i as usize] = if ((x * x + y * y + z * z) as f32).sqrt() < 15.0 {
                Block { solid: true }
            } else {
                Block { solid: false }
            };

            // Make the padding area empty, so that the algorithm does not cull the faces on the outside
            if x == 0 || x == 18 || y == 0 || y == 18 || z == 0 || z == 18 {
                chunk[i as usize] = Block { solid: false }
            };
        }

        let stage_buffer = GreedyQuadsBuffer::new(chunk.len());

        return Self {
            chunk,
            stage_buffer
        }
    }

    pub fn make_mesh(&mut self, voxel_size: f32) -> CPUMesh {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

        greedy_quads(
            &self.chunk,
            &ChunkShape {},
            [0,0,0],
            [17,17,17],
            &faces,
            &mut self.stage_buffer
        );

        let mut mesh = CPUMesh {
            verts: vec![],
            indxs: vec![]
        };
        for i in 0..6 {
            let face = faces[i];
            let quads = &self.stage_buffer.quads.groups[i];

            for quad in quads.iter() {
                let verts = face.quad_mesh_positions(quad, voxel_size);
                let indxs = face.quad_mesh_indices(mesh.verts.len() as u32);

                for vert in verts {
                    mesh.verts.push(Vertex { pos: vert });
                }
                for indx in indxs {
                    mesh.indxs.push(indx as Index);
                }
            }
        }

        return mesh
    }
}