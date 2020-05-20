use amethyst::ecs::{Component, DenseVecStorage};
use std::ops;

use super::Facing;

pub mod storage;

#[derive(Component)]
pub struct Chunk {
  // pub level: Entity,
  pub pos: ChunkPos,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct ChunkPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

impl ChunkPos {
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    ChunkPos { x, y, z }
  }
}

impl ops::Add<(i32, i32, i32)> for ChunkPos {
  type Output = Self;
  fn add(self, (x, y, z): (i32, i32, i32)) -> Self {
    ChunkPos {
      x: self.x + x,
      y: self.y + y,
      z: self.z + z,
    }
  }
}

impl ops::Sub<(i32, i32, i32)> for ChunkPos {
  type Output = Self;
  fn sub(self, (x, y, z): (i32, i32, i32)) -> Self {
    ChunkPos {
      x: self.x - x,
      y: self.y - y,
      z: self.z - z,
    }
  }
}

impl ops::Add<Facing> for ChunkPos {
  type Output = Self;
  fn add(self, face: Facing) -> Self {
    let vec: (i32, i32, i32) = face.into();
    self + vec
  }
}

impl ops::Sub<Facing> for ChunkPos {
  type Output = Self;
  fn sub(self, face: Facing) -> Self {
    let vec: (i32, i32, i32) = face.into();
    self - vec
  }
}
