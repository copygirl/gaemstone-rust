use self::Facing::*;
use std::{convert::TryFrom, ops};

pub use self::chunk::ChunkPos;
pub use self::mesh_generator::*;
pub use self::world_generator::*;

pub mod chunk;
mod mesh_generator;
mod world_generator;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BlockPos {
  pub x: i32,
  pub y: i32,
  pub z: i32,
}

impl BlockPos {
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    BlockPos { x, y, z }
  }
}

impl ops::Add<(i32, i32, i32)> for BlockPos {
  type Output = Self;
  fn add(self, (x, y, z): (i32, i32, i32)) -> Self {
    BlockPos {
      x: self.x + x,
      y: self.y + y,
      z: self.z + z,
    }
  }
}

impl ops::Sub<(i32, i32, i32)> for BlockPos {
  type Output = Self;
  fn sub(self, (x, y, z): (i32, i32, i32)) -> Self {
    BlockPos {
      x: self.x - x,
      y: self.y - y,
      z: self.z - z,
    }
  }
}

impl ops::Add<Facing> for BlockPos {
  type Output = Self;
  fn add(self, face: Facing) -> Self {
    let vec: (i32, i32, i32) = face.into();
    self + vec
  }
}

impl ops::Sub<Facing> for BlockPos {
  type Output = Self;
  fn sub(self, face: Facing) -> Self {
    let vec: (i32, i32, i32) = face.into();
    self - vec
  }
}

#[derive(Copy, Clone)]
pub enum Facing {
  /// Towards `+X`.
  East,
  /// Towards `-X`.
  West,
  /// Towards `+Y`.
  Up,
  /// Towards `-Y`.
  Down,
  /// Towards `+Z`.
  South,
  /// Towards `-Z`.
  North,
}

impl Facing {
  pub fn opposite(self) -> Self {
    match self {
      East => West,
      West => East,
      Up => Down,
      Down => Up,
      South => North,
      North => South,
    }
  }

  pub fn iter_all() -> impl Iterator<Item = Facing> {
    [East, West, Up, Down, South, North].iter().copied()
  }

  pub fn iter_horizontal() -> impl Iterator<Item = Facing> {
    [East, West, South, North].iter().copied()
  }

  pub fn iter_vertical() -> impl Iterator<Item = Facing> {
    [Up, Down].iter().copied()
  }
}

impl TryFrom<(i32, i32, i32)> for Facing {
  type Error = String;

  fn try_from(vec: (i32, i32, i32)) -> Result<Self, Self::Error> {
    match vec {
      (1, 0, 0) => Ok(East),
      (-1, 0, 0) => Ok(West),
      (0, 1, 0) => Ok(Up),
      (0, -1, 0) => Ok(Down),
      (0, 0, 1) => Ok(South),
      (0, 0, -1) => Ok(North),
      _ => Err(format!("Can't convert {:?} to Facing", vec)),
    }
  }
}

impl Into<(i32, i32, i32)> for Facing {
  fn into(self) -> (i32, i32, i32) {
    match self {
      East => (1, 0, 0),
      West => (-1, 0, 0),
      Up => (0, 1, 0),
      Down => (0, -1, 0),
      South => (0, 0, 1),
      North => (0, 0, -1),
    }
  }
}

impl ops::Mul<i32> for Facing {
  type Output = (i32, i32, i32);
  fn mul(self, factor: i32) -> (i32, i32, i32) {
    let (x, y, z): (i32, i32, i32) = self.into();
    (x * factor, y * factor, z * factor)
  }
}
