use block_mesh::ndshape::{ConstShape, ConstShape3u32};
use block_mesh::{MergeVoxel, Voxel};
use std::collections::HashMap;

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
    pub chunks: PosHash<[Block; ChunkShape::SIZE as usize]>
}
impl TerrainState {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
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
}