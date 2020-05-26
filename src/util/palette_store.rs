use bitvec::{bitvec, slice::AsBits, vec::BitVec};

const DEFAULT_CAPACITY: usize = 32;

/// This data structure contains a set amount of virtual elements which can be read using `get()`
/// and written using `set()` using a simple index bound by the size given to the palette store's
/// constructor. Based on ["Palette-based compression for chunked discrete voxel data"][post] by
/// /u/Longor1996, but adapted to work for a linear storage vector.
///
/// Behind the scenes, every distinct value of `T` is stored in a palette entry, and only the index
/// into that palette is stored, compactly, inside a bit vector.
///
/// [post]: https://www.reddit.com/r/VoxelGameDev/comments/9yu8qy/palettebased_compression_for_chunked_discrete/
///
/// # Example
///
/// ```
/// let mut store = PaletteStore::<u8>::new(16);
/// assert_eq!(store.get(8).unwrap(), Default::default());
///
/// store.set(8, 100u8).unwrap();
/// store.set(12, 50u8).unwrap();
/// assert_eq!(store.get(8).unwrap(), 100u8);
/// assert_eq!(store.get(12).unwrap(), 50u8);
///
/// store.set(12, 0u8).unwrap();
/// assert_eq!(store.get(12).unwrap(), 0u8);
///
/// assert!(store.get(16).is_err());
/// assert!(store.set(20, 0u8).is_err());
/// ```
pub struct PaletteStore<T: Default + Copy + Eq> {
  /// Number of virtual elements stored in this data structure.
  size: usize,
  /// Underlying bit vector, storing `bits_per_entry` bits for each virtual element
  /// that represent an index into `entries`. Its size is always `size * bits_per_entry`.
  bits: BitVec,
  /// Current number of bits for each virtual element in `bits`.
  bits_per_entry: usize,
  /// Vector which stores palette entries.
  entries: Vec<PaletteEntry<T>>,
  /// Number of palette entries currently in use (`ref_count > 0`).
  used: usize,
}

#[derive(Default, Copy, Clone)]
struct PaletteEntry<T> {
  value: T,
  ref_count: usize,
}

impl<T: Default + Copy + Eq> PaletteStore<T> {
  /// Creates a new palette store with the specified number of virtual elements.
  pub fn new(size: usize) -> Self {
    PaletteStore {
      size,
      bits: bitvec![],
      bits_per_entry: 0,
      entries: vec![],
      used: 0,
    }
  }

  /// Creates a new palette store with the specified number
  /// of virtual elements and capacity of palette entries.
  ///
  /// This is equivalent to calling:
  /// ```
  /// let mut storage = Self::new(size);
  /// storage.reserve(capacity);
  /// ```
  pub fn new_with_capacity(size: usize, capacity: usize) -> Self {
    let mut storage = Self::new(size);
    storage.reserve(capacity);
    storage
  }

  /// Gets the number of virtual elements stored in this data structure.
  #[inline]
  pub fn size(&self) -> usize {
    self.size
  }

  /// Gets the number of currently used palette entries.
  #[inline]
  pub fn used_entries(&self) -> usize {
    self.used
  }

  /// Gets the number of free palette entries, before the underlying bit vector needs to be resized.
  #[inline]
  pub fn free_entries(&self) -> usize {
    self.entries.len() - self.used_entries()
  }

  /// Reserves a number of additional palette entries on top of the current number of
  /// `used_entries()`. No effect if `additional` is smaller or equals to `free_entries()`.
  pub fn reserve(&mut self, additional: usize) {
    let req_capacity = self.used_entries() + additional;
    if req_capacity > self.entries.len() {
      let num_bits = integer_log2(req_capacity.next_power_of_two());
      self.set_bits_per_entry(num_bits);
    }
  }

  pub fn get(&self, index: usize) -> Result<T, &'static str> {
    if index >= self.size {
      Err("Out of bounds")
    } else {
      // SAFETY: Bounds already checked.
      unsafe { Ok(self.get_unchecked(index)) }
    }
  }

  pub fn set(&mut self, index: usize, value: T) -> Result<(), &'static str> {
    if index >= self.size {
      Err("Out of bounds")
    } else {
      // SAFETY: Bounds already checked.
      unsafe { Ok(self.set_unchecked(index, value)) }
    }
  }

  pub unsafe fn get_unchecked(&self, index: usize) -> T {
    if self.used == 0 {
      // If no palette entries are currently being used (such as when the
      // palette store was just created), just return the default value.
      Default::default()
    } else {
      let palette_index = self.get_palette_index(index);
      self.entries[palette_index].value
    }
  }

  pub unsafe fn set_unchecked(&mut self, index: usize, value: T) {
    if self.used == 0 {
      // If no palette entries are currently being used (such as when the palette
      // store was just created), and the virtual element is being set to the default
      // value (which would not change what is returned by `get()`), do nothing.
      if value == Default::default() {
        return;
      }
    } else {
      let palette_index = self.get_palette_index(index);
      let mut current = &mut self.entries[palette_index];

      // If nothing changes, don't bother.
      if value == current.value {
        return;
      }

      // Reduce the `ref_count` in the current palette entry.
      // If this hits 0, the entry is free to be used by new values, except
      // for the first palette entry, which represents the default value.
      current.ref_count -= 1;
      if current.ref_count == 0 && palette_index > 0 {
        current.value = Default::default();
        self.used -= 1;
      }

      // Find an existing palette entry for the new value being set.
      // If successful, replace the old palette index in `bits` with its index.
      if let Some(i) = self.entries.iter().position(|e| e.value == value) {
        self.set_palette_index(index, i);
        self.entries[i].ref_count += 1;
        return;
      }

      // Need to re-borrow `entries`, else we can't `iter()` on it earlier.
      let mut current = &mut self.entries[palette_index];
      // If it just so happens that we freed up the old palette
      // entry, we can replace it to refer to the new value.
      if current.ref_count == 0 && palette_index > 0 {
        current.value = value;
        current.ref_count = 1;
        self.used += 1;
        return;
      }
    }

    // Get a free palette entry, expanding `bits` and `entries` if needed.
    let palette_index = self.get_free_palette_index();
    self.entries[palette_index] = PaletteEntry {
      value,
      ref_count: 1,
    };
    self.set_palette_index(index, palette_index);
    self.used += 1;
  }

  /// Gets the index of a free palette entry, reserving additional entries if required.
  fn get_free_palette_index(&mut self) -> usize {
    // Test to see if there should be a free palette entry and, if so, return its index.
    if self.free_entries() > 0 {
      self.entries.iter().position(|e| e.ref_count == 0).unwrap()
    } else {
      // If `entries` is empty, initialize capacity to DEFAULT_CAPACITY.
      if self.entries.is_empty() {
        self.reserve(DEFAULT_CAPACITY);
        // Index 0 is already in use by the default value, so return 1 instead.
        1
      // Otherwise, reserve at least one additional element. This will cause the capacity to double,
      // as one additional bit will be required to store the additional palette entries.
      } else {
        let previous_capacity = self.entries.len();
        self.reserve(1);
        previous_capacity // What was previously the maximum capacity is now a free palette index!
      }
    }
  }

  fn set_bits_per_entry(&mut self, num_bits: usize) {
    // If nothing changes, don't bother.
    if num_bits == self.bits_per_entry {
      return;
    // If `bits_per_entry` is being set to zero, reset the whole palette store.
    } else if num_bits == 0 {
      self.bits = bitvec![];
      self.entries = vec![];
      self.used = 0;
    // If palette entries is empty (such as when the palette store was just created), initialize
    // everything to its default state. This will cause a single palette entry to be used that
    // takes up all of the palette stores's virtual elements (as it has an all-zero bit pattern).
    } else if self.entries.is_empty() {
      self.bits = bitvec![0; self.size * num_bits];
      self.entries = vec![Default::default(); 1 << num_bits];
      self.entries[0].ref_count = self.size;
      self.used = 1;
    // If `bits_per_entry` grows, grow the underlying bits and palette vectors.
    } else if num_bits > self.bits_per_entry {
      // Build new bit vector, going through each element slice and copying it from the old data.
      let mut new_bits = bitvec![0; self.size * num_bits];
      let skip_bits = num_bits - self.bits_per_entry;
      for (old, new) in self
        .bits
        .chunks(self.bits_per_entry)
        .zip(new_bits.chunks_mut(num_bits))
      {
        new[skip_bits..].copy_from_slice(old);
      }
      self.bits = new_bits;

      // Expand the palette to new capacity.
      self.entries.resize(1 << num_bits, Default::default());
    // If `bits_per_entry` shrinks, reorganize palette entries and recreate underlying bit vector.
    } else {
      assert!(
        self.used_entries() > (1 << num_bits),
        "Attempted to shrink, but can't fit currently used entries"
      );

      // Create a lookup table to map old palette indices to new ones.
      let mut old_to_new_indices = vec![0usize; self.entries.len()];

      // Reorganize palette entries, compacting them at the beginning of the
      // vector so it can later be truncated, dropping only unused entries.
      let mut counter = 1;
      for i in 1..self.entries.len() {
        if self.entries[i].ref_count > 0 {
          if counter < i {
            self.entries[counter] = self.entries[i];
          }
          old_to_new_indices[i] = counter;
          counter += 1;
        }
      }
      // Truncate palette to new capacity.
      self.entries.truncate(1 << num_bits);

      // Build new bit vector, going through each entry and populating it
      // with the new palette index looked up using `old_to_new_indices`.
      let mut new_bits = bitvec![0; self.size * num_bits];
      for (i, new) in new_bits.chunks_mut(num_bits).enumerate() {
        let new_index = old_to_new_indices[self.get_palette_index(i)];
        new.copy_from_slice(&new_index.bits()[..num_bits]);
      }
      self.bits = new_bits;
    }
    self.bits_per_entry = num_bits;
  }

  /// Gets the palette index for the virtual element stored
  /// at the specified index, by decoding it from `bits`.
  fn get_palette_index(&self, index: usize) -> usize {
    let index = index * self.bits_per_entry;
    let slice = &self.bits[index..(index + self.bits_per_entry)];
    let mut value = 0usize;
    value.bits_mut()[..self.bits_per_entry].copy_from_slice(slice);
    value
  }

  /// Sets the palette index for the virtual element stored
  /// at the specified index, by encoding it into `bits`.
  fn set_palette_index(&mut self, index: usize, value: usize) {
    let index = index * self.bits_per_entry;
    let slice = &mut self.bits[index..(index + self.bits_per_entry)];
    slice.copy_from_slice(&value.bits()[..self.bits_per_entry]);
  }
}

fn integer_log2(mut i: usize) -> usize {
  let mut power = 0;
  i >>= 1;
  while i != 0 {
    i >>= 1;
    power += 1;
  }
  power
}
