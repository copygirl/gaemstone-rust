use amethyst::ecs::{Component, DenseVecStorage};
use std::{convert::TryFrom, error::Error, fmt, ops};

use super::Facing;

pub mod storage;

pub const CHUNK_LENGTH_BITS: usize = 4;
pub const CHUNK_LENGTH: usize = 1 << CHUNK_LENGTH_BITS;
pub const CHUNK_SIZE: usize = 1 << (CHUNK_LENGTH_BITS * 3);

#[derive(Component)]
pub struct Chunk {
  // pub level: Entity,
  pub pos: ChunkPos,
}

bitflags! {
  #[derive(Default)]
  pub struct ChunkState: u8 {
    const EXISTS = 0b00000001;
    const GENERATED = 0b00000010;
    const MESH_UPDATED = 0b00000100;
  }
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

const BIT_MASK: i32 = !(!0 << CHUNK_LENGTH_BITS);

// TODO: With `u16` being the base type, `Index` can only support `CHUNK_LENGTH` up to 32 (5 bits).
//       Consider encoding this using "Z-order curve"? Not sure what the benefits are.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Index(u16);

impl Index {
  pub fn new(x: i32, y: i32, z: i32) -> Result<Self, BoundsError> {
    if (x & !BIT_MASK == 0) && (y & !BIT_MASK == 0) && (z & !BIT_MASK == 0) {
      // SAFETY: Bounds already checked.
      unsafe { Ok(Self::new_unchecked(x, y, z)) }
    } else {
      Err(BoundsError(x, y, z))
    }
  }

  pub unsafe fn new_unchecked(x: i32, y: i32, z: i32) -> Self {
    Index((x | (y << CHUNK_LENGTH_BITS) | (z << (CHUNK_LENGTH_BITS * 2))) as u16)
  }

  pub fn x(&self) -> i32 {
    (self.0 as i32) & BIT_MASK
  }

  pub fn y(&self) -> i32 {
    (self.0 as i32 >> CHUNK_LENGTH_BITS) & BIT_MASK
  }

  pub fn z(&self) -> i32 {
    (self.0 as i32 >> (CHUNK_LENGTH_BITS * 2)) & BIT_MASK
  }

  #[inline]
  pub fn raw_index(&self) -> u16 {
    self.0
  }
}

impl TryFrom<(i32, i32, i32)> for Index {
  type Error = BoundsError;
  fn try_from((x, y, z): (i32, i32, i32)) -> Result<Self, Self::Error> {
    Self::new(x, y, z)
  }
}

impl Into<(i32, i32, i32)> for Index {
  fn into(self) -> (i32, i32, i32) {
    (self.x(), self.y(), self.z())
  }
}

impl fmt::Debug for Index {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "(index={}, x={}, y={}, z={})",
      self.0,
      self.x(),
      self.y(),
      self.z()
    )
  }
}

impl fmt::Display for Index {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "({}, {}, {})", self.x(), self.y(), self.z())
  }
}

#[derive(Debug)]
pub struct BoundsError(i32, i32, i32);

impl Error for BoundsError {}

impl fmt::Display for BoundsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(
      f,
      "Index ({}, {}, {}) outside chunk bounds",
      self.0, self.1, self.2
    )
  }
}
