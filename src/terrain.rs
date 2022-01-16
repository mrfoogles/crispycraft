use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{greedy_quads, GreedyQuadsBuffer, MergeVoxel, Voxel, RIGHT_HANDED_Y_UP_CONFIG};

use crate::game::{Vertex, Index, CPUMesh};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Block {
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
pub type ChunkShape = ConstShape3u32<18, 18, 18>;

pub fn generate(chunk: &mut [Block; ChunkShape::SIZE as usize], radius: f32) {
    for i in 0..ChunkShape::SIZE {
        let [x, y, z] = ChunkShape::delinearize(i);

        // Generate a sphere? I did not write these 5 lines
        chunk[i as usize] = if ((x * x + y * y + z * z) as f32).sqrt() < radius {
            Block { solid: true }
        } else {
            Block { solid: false }
        };

        // Make the padding area empty, so that the algorithm does not cull the faces on the outside
        if x == 0 || x == 17 || y == 0 || y == 17 || z == 0 || z == 17 {
            chunk[i as usize] = Block { solid: false }
        };
    }
}

pub struct TerrainState {
    pub chunk: [Block; ChunkShape::SIZE as usize],
    stage_buffer: GreedyQuadsBuffer
}
impl TerrainState {
    pub fn new() -> Self {
        // Empty
        let mut chunk = [Block { solid: false }; ChunkShape::SIZE as usize];
        // Fill with radius 15. sphere
        generate(&mut chunk, 15.);

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
            [0,0,0], // start (include padding)
            [17,17,17], // end (include padding)
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