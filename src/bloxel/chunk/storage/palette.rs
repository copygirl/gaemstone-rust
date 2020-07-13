use {
  super::{
    super::{Index, CHUNK_SIZE},
    BlockData, StorageImpl,
  },
  crate::util::PaletteStore,
};

pub struct PaletteStorageImpl<T: BlockData> {
  pub data: PaletteStore<T>,
}

impl<T: BlockData> PaletteStorageImpl<T> {
  pub fn new() -> Self {
    PaletteStorageImpl {
      data: PaletteStore::new(CHUNK_SIZE),
    }
  }

  pub fn new_with_capacity(capacity: usize) -> Self {
    let mut storage = Self::new();
    storage.data.reserve(capacity);
    storage
  }
}

impl<T: BlockData> StorageImpl<T> for PaletteStorageImpl<T> {
  fn get(&self, index: Index) -> T {
    // SAFETY: Bounds already satisfied by chunk size.
    unsafe { self.data.get_unchecked(index.raw_index() as usize) }
  }

  fn set(&mut self, index: Index, value: T) {
    // SAFETY: Bounds already satisfied by chunk size.
    unsafe { self.data.set_unchecked(index.raw_index() as usize, value) }
  }
}
