use super::{BlockData, Bounds, BoundsError, StorageImpl};
use crate::util::PaletteStore;

pub struct PaletteStorageImpl<T: BlockData> {
  bounds: Bounds,
  pub data: PaletteStore<T>,
}

impl<T: BlockData> PaletteStorageImpl<T> {
  pub fn new(width: u32, height: u32, depth: u32) -> Self {
    let bounds = Bounds::new(width, height, depth);
    PaletteStorageImpl {
      bounds: bounds,
      data: PaletteStore::new(bounds.size()),
    }
  }

  pub fn new_with_capacity(width: u32, height: u32, depth: u32, capacity: usize) -> Self {
    let mut storage = Self::new(width, height, depth);
    storage.data.reserve(capacity);
    storage
  }
}

impl<T: BlockData> StorageImpl<T> for PaletteStorageImpl<T> {
  fn bounds(&self) -> Bounds {
    self.bounds
  }

  fn get(&self, x: i32, y: i32, z: i32) -> Result<T, BoundsError> {
    let index = self.bounds.get_index(x, y, z)?;
    // SAFETY: Bounds already checked.
    unsafe { Ok(self.data.get_unchecked(index)) }
  }

  fn set(&mut self, x: i32, y: i32, z: i32, value: T) -> Result<(), BoundsError> {
    let index = self.bounds.get_index(x, y, z)?;
    // SAFETY: Bounds already checked.
    unsafe { self.data.set_unchecked(index, value) }
    Ok(())
  }
}
