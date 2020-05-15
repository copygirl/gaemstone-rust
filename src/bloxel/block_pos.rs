use std::ops;

use super::facing::Facing;

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct BlockPos {
  x: i32,
  y: i32,
  z: i32,
}

impl BlockPos {
  pub fn new(x: i32, y: i32, z: i32) -> Self {
    BlockPos { x, y, z }
  }

  pub fn x(self) -> i32 {
    self.x
  }

  pub fn y(self) -> i32 {
    self.y
  }

  pub fn z(self) -> i32 {
    self.z
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
