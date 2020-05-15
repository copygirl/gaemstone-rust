use amethyst::ecs::{Component, DenseVecStorage, Entity};

pub use self::{block_pos::*, chunk_pos::*, chunk_storage::*, facing::*};

mod block_pos;
mod chunk_pos;
mod chunk_storage;
mod facing;

#[derive(Component)]
pub struct Chunk {
  level: Entity,
  pos: ChunkPos,
}
