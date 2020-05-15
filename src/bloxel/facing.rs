use std::{convert::TryFrom, ops};

use self::Facing::*;

#[derive(Copy, Clone)]
pub enum Facing {
  East,  // +X
  West,  // -X
  Up,    // +Y
  Down,  // -Y
  South, // +Z
  North, // -Z
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
