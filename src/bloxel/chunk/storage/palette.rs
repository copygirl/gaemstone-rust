use bitvec::{slice::AsBits, vec::BitVec};

use super::{BlockData, Bounds, BoundsError, StorageImpl};

/// Palette based storage for arbitrary data in a chunk format which can be of varied size.
/// Based on ["Palette-based compression for chunked discrete voxel data"][post] by /u/Longor1996.
///
/// Operates by keeping a variable-size palette around, which can be grown and shrunk, but its size
/// will always be a power of 2. A bit vector then stores the indices into the palette for each
/// "block" index in the storage, each taking up just as many bits as needed.
///
/// [post]: https://www.reddit.com/r/VoxelGameDev/comments/9yu8qy/palettebased_compression_for_chunked_discrete/
///
/// # Example
///
/// ```
/// let mut storage = ChunkPaletteStorage::<u8>::new(16, 16, 16);
/// assert_eq!(storage.get(0, 0, 0), Default::default());
///
/// storage.set(4, 8, 10, 100u8).unwrap();
/// storage.set(8, 14, 12, 50u8).unwrap();
/// assert_eq!(storage.get(4, 8, 10).unwrap(), 100u8);
/// assert_eq!(storage.get(8, 14, 12).unwrap(), 50u8);
///
/// storage.set(4, 8, 10, 0u8).unwrap();
/// assert_eq!(storage.get(4, 8, 10).unwrap(), 0u8);
///
/// assert!(storage.get(8, 16, 8).is_err());
/// assert!(storage.get(0, -1, 0).is_err());
/// ```
pub struct PaletteStorageImpl<T: BlockData> {
  bounds: Bounds,
  /// Bit vector containing variable-sized bit slices,
  /// each representing an index into the `palette` vector.
  /// The size of this vector is `indices_len * width*height*depth`.
  data: BitVec,
  /// The size (in bits) of each bit slice contained in `data`.
  indices_len: usize,
  /// Vector containing palette entries which point to the actual
  /// data type, and store how many times they occur in this storage.
  palette: Vec<PaletteEntry<T>>,
  /// The number of palettes in `palette` currently in use (where `ref_count > 0`).
  used_palettes: usize,
}

impl<T: BlockData> PaletteStorageImpl<T> {
  pub fn new(width: u32, height: u32, depth: u32) -> Self {
    let bounds = Bounds::new(width, height, depth);
    let size = bounds.size();
    PaletteStorageImpl {
      bounds: bounds,
      data: BitVec::repeat(false, size),
      palette: vec![
        PaletteEntry {
          value: Default::default(),
          ref_count: size,
        },
        Default::default(),
      ],
      used_palettes: 1,
      indices_len: 1,
    }
  }

  /// Gets an unused palette entry, growing the `palette` vector if needed.
  fn get_palette_entry(&mut self) -> usize {
    if self.used_palettes < self.palette.len() {
      self.palette.iter().position(|x| x.ref_count == 0).unwrap()
    } else {
      self.grow_palette();
      self.get_palette_entry()
    }
  }

  fn grow_palette(&mut self) {
    // Build new data bit vector, going through each slice and copying it from the old data.
    let len = self.indices_len;
    let mut new_data = BitVec::repeat(false, self.bounds.size() * (len + 1));
    for (old, new) in self.data.chunks(len).zip(new_data.chunks_mut(len + 1)) {
      new[1..].copy_from_slice(old);
    }
    self.data = new_data;

    // Expand palette to twice the size.
    let len = self.palette.len() << 1;
    self.palette.resize(len, Default::default());

    self.indices_len += 1;
  }

  fn shrink_palette(&mut self) {
    if self.used_palettes > self.used_palettes.next_power_of_two() / 2 {
      return;
    }

    // Create a lookup table to map old palette indices to new ones.
    let mut old_to_new_indices = vec![0usize; self.palette.len()];

    // Build the new palette (which has half the size), moving old palette
    // entries to the new ones only if they are being used (`ref_count > 0`).
    let mut palette_counter = 0;
    let mut new_palette = vec![PaletteEntry::default(); self.palette.len() >> 1];
    for (i, entry) in self.palette.iter().enumerate() {
      if entry.ref_count == 0 {
        continue;
      }
      old_to_new_indices[i] = palette_counter;
      new_palette[palette_counter] = *entry;
      palette_counter += 1;
    }

    // Build new data bit vector, going through each entry and populating
    // it with the new palette index looked up using `old_to_new_indices`.
    let mut new_data = BitVec::repeat(false, self.bounds.size() * (self.indices_len - 1));
    for (i, new) in new_data.chunks_mut(self.indices_len - 1).enumerate() {
      let new_index = old_to_new_indices[self.get_palette_index(i)];
      new.copy_from_slice(&new_index.bits()[0..new.len()]);
    }

    self.data = new_data;
    self.palette = new_palette;
    self.indices_len -= 1;
  }

  fn get_palette_index(&self, index: usize) -> usize {
    let slice = &self.data[index..(index + self.indices_len)];
    let mut palette_index = 0usize;
    palette_index.bits_mut()[0..self.indices_len].copy_from_slice(slice);
    palette_index
  }

  fn set_palette_index(&mut self, index: usize, value: usize) {
    let slice = &mut self.data[index..(index + self.indices_len)];
    slice.copy_from_slice(&value.bits()[0..self.indices_len]);
  }
}

impl<T: BlockData> StorageImpl<T> for PaletteStorageImpl<T> {
  fn bounds(&self) -> Bounds {
    self.bounds
  }

  fn get(&self, x: i32, y: i32, z: i32) -> Result<T, BoundsError> {
    let index = self.bounds.get_index(x, y, z)? * self.indices_len;
    let palette_index = self.get_palette_index(index);
    Ok(self.palette[palette_index].value)
  }

  fn set(&mut self, x: i32, y: i32, z: i32, value: T) -> Result<(), BoundsError> {
    let index = self.bounds.get_index(x, y, z)? * self.indices_len;
    let palette_index = self.get_palette_index(index);
    let mut current = &mut self.palette[palette_index];

    // If nothing changes, don't bother.
    if value == current.value {
      return Ok(());
    }

    // Reduce the `ref_count` in the current palette entry.
    // If this hits 0, the entry is free to be used by new values.
    current.ref_count -= 1;
    if current.ref_count == 0 {
      self.used_palettes -= 1;
    }

    // Find an existing palette entry for the new value being set.
    // If successful, replace the old palette index in `data` with it.
    if let Some(i) = self.palette.iter().position(|x| value == x.value) {
      self.set_palette_index(index, i);
      self.palette[i].ref_count += 1;
      return Ok(());
    }

    // Need to re-borrow `palette`, else we can't `iter()` on it earlier.
    let mut current = &mut self.palette[palette_index];
    // If it just so happens that we freed up the old palette
    // entry, we can replace it to refer to the new value.
    if current.ref_count == 0 {
      current.value = value;
      current.ref_count = 1;
      self.used_palettes += 1;
      return Ok(());
    }

    // Get a free palette entry, expanding `palette` if needed.
    let palette_index = self.get_palette_entry();
    self.palette[palette_index] = PaletteEntry {
      value,
      ref_count: 1,
    };
    self.set_palette_index(index, palette_index);
    self.used_palettes += 1;
    Ok(())
  }
}

#[derive(Default, Copy, Clone)]
struct PaletteEntry<T: Default + Copy + Eq> {
  value: T,
  ref_count: usize,
}
