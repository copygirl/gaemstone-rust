use {
  super::Index,
  amethyst::ecs::{Component, DenseVecStorage},
  std::sync::RwLock,
};

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

  /// Attempts to get a value from this storage from the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  pub fn get(&self, index: Index) -> T {
    self.storage.read().unwrap().get(index)
  }

  /// Attempts to set a value from this storage at the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  pub fn set(&mut self, index: Index, value: T) {
    self.storage.write().unwrap().set(index, value)
  }
}

pub trait StorageImpl<T: BlockData> {
  /// Attempts to get a value from this storage from the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  fn get(&self, index: Index) -> T;

  /// Attempts to set a value from this storage at the specified relative coordinates.
  /// Returns `Err(BoundsError)` if the coordinates are outside the bounds of the storage.
  fn set(&mut self, index: Index, value: T);
}
