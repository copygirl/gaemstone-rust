use amethyst::ecs::{Component, DenseVecStorage};
use std::{convert::TryInto, error::Error, fmt, sync::RwLock};

pub use palette::*;

mod palette;

pub trait BlockData: Default + Copy + Eq + 'static {}
impl<T: Default + Copy + Eq + 'static> BlockData for T {}

#[derive(Component)]
pub struct ChunkStorage<T: BlockData> {
  storage: RwLock<Box<dyn StorageImpl<T>>>,
}

unsafe impl<T: BlockData> Send for ChunkStorage<T> {}
unsafe impl<T: BlockData> Sync for ChunkStorage<T> {}

impl<T: BlockData> ChunkStorage<T> {
  pub fn new<S: StorageImpl<T> + 'static>(storage: S) -> Self {
    ChunkStorage {
      storage: RwLock::new(Box::new(storage)),
    }
  }

  pub fn bounds(&self) -> Bounds {
    self.storage.read().unwrap().bounds()
  }

  /// Attempts to get a value from this storage from the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  pub fn get(&self, x: i32, y: i32, z: i32) -> Result<T, BoundsError> {
    self.storage.read().unwrap().get(x, y, z)
  }

  /// Attempts to set a value from this storage at the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  pub fn set(&mut self, x: i32, y: i32, z: i32, value: T) -> Result<(), BoundsError> {
    self.storage.write().unwrap().set(x, y, z, value)
  }
}

pub trait StorageImpl<T: BlockData> {
  fn bounds(&self) -> Bounds;

  /// Attempts to get a value from this storage from the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  fn get(&self, x: i32, y: i32, z: i32) -> Result<T, BoundsError>;

  /// Attempts to set a value from this storage at the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  fn set(&mut self, x: i32, y: i32, z: i32, value: T) -> Result<(), BoundsError>;
}

#[derive(Copy, Clone)]
pub enum Bounds {
  PowerOfTwo(u32),
  Other(u32, u32, u32),
}

impl Bounds {
  fn new(width: u32, height: u32, depth: u32) -> Self {
    assert!(width > 0, "width must be > 0");
    assert!(height > 0, "height must be > 0");
    assert!(depth > 0, "depth must be > 0");
    // If width, height and depth are the same and a power of two, the indexing method can
    // use bitwise operations, which should be more efficient than the "default" indexing.
    if width == height && height == depth && width.is_power_of_two() {
      Bounds::PowerOfTwo(integer_log2(width))
    } else {
      Bounds::Other(width, height, depth)
    }
  }

  pub fn width(&self) -> u32 {
    match self {
      Self::PowerOfTwo(p) => 1 << *p,
      Self::Other(w, _, _) => *w,
    }
  }

  pub fn height(&self) -> u32 {
    match self {
      Self::PowerOfTwo(p) => 1 << *p,
      Self::Other(_, h, _) => *h,
    }
  }

  pub fn depth(&self) -> u32 {
    match self {
      Self::PowerOfTwo(p) => 1 << *p,
      Self::Other(_, _, d) => *d,
    }
  }

  /// Gets the total amount of entries contained within these bounds (`width*height*depth`).
  pub fn size(&self) -> usize {
    match self {
      Self::PowerOfTwo(p) => (1usize << p).pow(3),
      Self::Other(w, h, d) => (w * h * d).try_into().unwrap(),
    }
  }

  /// Returns whether the specified coords are valid, and within these bounds.
  pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
    match self {
      Self::PowerOfTwo(p) => (x >> p) | (y >> p) | (z >> p) == 0,
      Self::Other(w, h, d) => {
        (x >= 0 && x < (*w as i32)) && (y >= 0 && y < (*h as i32)) && (z >= 0 && z < (*d as i32))
      }
    }
  }

  fn get_index(&self, x: i32, y: i32, z: i32) -> Result<usize, BoundsError> {
    if self.contains(x, y, z) {
      // SAFETY: Coords were already bounds checked.
      unsafe { Ok(self.get_index_unchecked(x, y, z)) }
    } else {
      Err(BoundsError {
        index: (x, y, z),
        bounds: (self.width(), self.height(), self.depth()),
      })
    }
  }

  unsafe fn get_index_unchecked(&self, x: i32, y: i32, z: i32) -> usize {
    match self {
      Self::PowerOfTwo(p) => x | (y << p) | (z << (p << 1)),
      Self::Other(w, h, _) => x + y * (*w as i32) + z * (*w as i32) * (*h as i32),
    }
    .try_into()
    .unwrap()
  }
}

fn integer_log2(mut i: u32) -> u32 {
  let mut power = 0;
  i >>= 1;
  while i != 0 {
    i >>= 1;
    power += 1;
  }
  power
}

#[derive(Debug)]
pub struct BoundsError {
  index: (i32, i32, i32),
  bounds: (u32, u32, u32),
}

impl Error for BoundsError {}

impl fmt::Display for BoundsError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "Index {:?} outside bounds {:?}", self.index, self.bounds)
  }
}
