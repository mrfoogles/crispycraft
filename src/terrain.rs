use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, RIGHT_HANDED_Y_UP_CONFIG};
use std::collections::HashMap;

use crate::game::{Vertex, Index, CPUMesh};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Block {
    pub solid: bool
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
pub const SIZE: u32 = 16;
pub type ChunkShape = ConstShape3u32<{SIZE + 2},{SIZE + 2},{SIZE + 2}>;
pub type ChunkPos = [i32; 3];
pub type PosHash<T> = std::collections::HashMap<ChunkPos, T>;

pub struct TerrainState {
    pub chunks: PosHash<[Block; ChunkShape::SIZE as usize]>,
    stage_buffer: GreedyQuadsBuffer
}
impl TerrainState {
    pub fn new() -> Self {
        let stage_buffer = GreedyQuadsBuffer::new(ChunkShape::SIZE as usize);

        Self {
            chunks: HashMap::new(),
            stage_buffer
        }
    }

    pub fn set_chunk<F: Fn([i32; 3], [i32; 3]) -> Block>(&mut self, pos: ChunkPos, func: F) {
        self.chunks.entry(pos).or_insert([Block { solid: false }; ChunkShape::SIZE as usize])
            .iter_mut().enumerate().for_each(|(i, block)| {
                let local = ChunkShape::delinearize(i as u32);
                *block = func([local[0] as i32, local[1] as i32, local[2] as i32], [
                    local[0] as i32 + pos[0] * SIZE as i32,
                    local[1] as i32 + pos[1] * SIZE as i32,
                    local[2] as i32 + pos[2] * SIZE as i32
                ]);
            });
    }

    pub fn make_mesh(&mut self, pos: ChunkPos, voxel_size: f32) -> Option<CPUMesh<Vertex>> {
        let faces = RIGHT_HANDED_Y_UP_CONFIG.faces;

        greedy_quads(
            self.chunks.get(&pos)?,
            &ChunkShape {},
            [0,0,0], // start (include padding)
            [SIZE+1,SIZE+1,SIZE+1], // end (include padding)
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
                        vert[0] + pos[0] as f32 * SIZE as f32,
                        vert[1] + pos[1] as f32 * SIZE as f32,
                        vert[2] + pos[2] as f32 * SIZE as f32
                    ] });
                }
                for indx in indxs {
                    mesh.indxs.push(indx as Index);
                }
            }
        }

        Some(mesh)
    }
}