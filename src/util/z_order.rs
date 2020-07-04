#![macro_use]

use {
  super::z_order_store::*,
  std::{
    cmp::Ordering,
    {fmt::Debug, ops::*},
  },
};

/// This struct wraps a primitive integer which represents an index into a space-filling curve
/// called [Z-Order Curve]. Often, this is also referred to as Morton order, code, or encoding.
/// This implementation purely focuses on 3 dimensions.
///
/// [Z-Order Curve]: https://en.wikipedia.org/wiki/Z-order_curve
///
/// By interleaving the 3 sub-elements into a single integer, some amount of packing can be
/// achieved, at the loss of some bits per elements. For example, with a 64 bit integer, 21 bits per
/// elements are available (`2_097_152` distinct values), which may be enough to represent block
/// coordinates in a bloxel game world.
///
/// Index | Element | Bits | Min. Value | Max. Value
/// :----:|:-------:|-----:|-----------:|----------:
/// `u16` |  `u8`   |    5 |          0 |         31
/// `u32` |  `u16`  |   10 |          0 |       1023
/// `u64` |  `u32`  |   21 |          0 |    2097151
/// `i16` |  `i8`   |    5 |        -16 |         15
/// `i32` |  `i16`  |   10 |       -512 |        511
/// `i64` |  `i32`  |   21 |   -1048576 |    1048575
///
/// One upside of encoding separate coordinates into a single Z-Order index is that it can then be
/// effectively used to index into octrees, and certain operations such as bitwise shifting are
/// quite useful.
///
/// # Examples
///
/// ```
/// let o = ZOrder::<i32>::new(13, -8, 1).unwrap();
/// // Can be destructed into a `(i32, i32, i32)` tuple:
/// let (x, y, z) = o.into();
/// assert_eq!((x, y, z), (13, -8, 1));
///
/// // Alternatively, only one element can be extracted:
/// assert_eq!(o.x(), 13);
/// assert_eq!(o.decode(2), 1);
///
/// // And now, let's try some of those bitwise operations:
/// assert_eq!((o >> 1).into(), (x >> 1, y >> 1, z >> 1));
/// assert_eq!((o << 2).into(), (x << 2, y << 2, z << 2));
/// assert_eq!(o & ZOrder::from_raw(0b111), ZOrder::from_raw(0b101));
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct ZOrder<T: ZOrderStore = i64>(T);

impl<T: ZOrderStore> ZOrder<T> {
  pub fn new(x: T::ElementType, y: T::ElementType, z: T::ElementType) -> Option<Self> {
    if (x >= T::ELEMENT_MIN && y >= T::ELEMENT_MIN && z >= T::ELEMENT_MIN)
      && (x <= T::ELEMENT_MAX && y <= T::ELEMENT_MAX && z <= T::ELEMENT_MAX)
    {
      // SAFETY: Bounds have already been checked.
      Some(unsafe { Self::new_unchecked(x, y, z) })
    } else {
      None
    }
  }

  pub unsafe fn new_unchecked(x: T::ElementType, y: T::ElementType, z: T::ElementType) -> Self {
    Self(T::split(x) | T::split(y) << 1 | T::split(z) << 2)
  }

  pub fn from_raw(order: T) -> Self {
    // Bit mask ensures that no bits outside the usable range are set.
    Self(order & !(!T::ZERO << T::MAX_USABLE_BITS))
  }

  pub fn raw(self) -> T {
    self.0
  }

  pub fn decode(self, i: usize) -> T::ElementType {
    assert!(i < 3);
    if T::SIGNED {
      (T::get(self.0 >> i) << T::SIGN_SHIFT) >> T::SIGN_SHIFT
    } else {
      T::get(self.0 >> i)
    }
  }

  pub fn into(self) -> (T::ElementType, T::ElementType, T::ElementType) {
    (self.x(), self.y(), self.z())
  }

  pub fn x(self) -> T::ElementType {
    self.decode(0)
  }
  pub fn y(self) -> T::ElementType {
    self.decode(1)
  }
  pub fn z(self) -> T::ElementType {
    self.decode(2)
  }

  pub fn inc_x(self) -> Self {
    let x_sum = (self.0 | T::YZ_MASK) + T::ONE;
    Self((x_sum & T::X_MASK) | (self.0 & T::YZ_MASK))
  }
  pub fn inc_y(self) -> Self {
    let y_sum = (self.0 | T::XZ_MASK) + (T::ONE << 1);
    Self((y_sum & T::Y_MASK) | (self.0 & T::XZ_MASK))
  }
  pub fn inc_z(self) -> Self {
    let z_sum = (self.0 | T::XY_MASK) + (T::ONE << 2);
    Self((z_sum & T::Z_MASK) | (self.0 & T::XY_MASK))
  }

  pub fn dec_x(self) -> Self {
    let x_diff = (self.0 & T::X_MASK) - T::ONE;
    Self((x_diff & T::X_MASK) | (self.0 & T::YZ_MASK))
  }
  pub fn dec_y(self) -> Self {
    let y_diff = (self.0 & T::Y_MASK) - (T::ONE << 1);
    Self((y_diff & T::Y_MASK) | (self.0 & T::XZ_MASK))
  }
  pub fn dec_z(self) -> Self {
    let z_diff = (self.0 & T::Z_MASK) - (T::ONE << 2);
    Self((z_diff & T::Z_MASK) | (self.0 & T::XY_MASK))
  }
}

impl<T: ZOrderStore> Into<(T::ElementType, T::ElementType, T::ElementType)> for ZOrder<T> {
  fn into(self) -> (T::ElementType, T::ElementType, T::ElementType) {
    (self.x(), self.y(), self.z())
  }
}

impl<T: ZOrderStore> Ord for ZOrder<T> {
  fn cmp(&self, rhs: &Self) -> Ordering {
    if T::SIGNED {
      // Invert sign bits so negative orders come before positive. Need to do this
      // because the most significant bits (like the actual sign bit) are always 0.
      let mask = !(!T::ZERO << 3) << (T::MAX_USABLE_BITS - 3);
      (self.0 ^ mask).cmp(&(rhs.0 ^ mask))
    } else {
      self.0.cmp(&rhs.0)
    }
  }
}
impl<T: ZOrderStore> PartialOrd for ZOrder<T> {
  fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
    Some(self.cmp(&rhs))
  }
}

impl<T: ZOrderStore> Add<Self> for ZOrder<T> {
  type Output = Self;
  fn add(self, rhs: Self) -> Self::Output {
    let x_sum = (self.0 | T::YZ_MASK) + (rhs.0 & T::X_MASK);
    let y_sum = (self.0 | T::XZ_MASK) + (rhs.0 & T::Y_MASK);
    let z_sum = (self.0 | T::XY_MASK) + (rhs.0 & T::Z_MASK);
    let sum = (x_sum & T::X_MASK) | (y_sum & T::Y_MASK) | (z_sum & T::Z_MASK);
    Self(sum & !(!T::ZERO << T::MAX_USABLE_BITS))
  }
}

impl<T: ZOrderStore> Sub<Self> for ZOrder<T> {
  type Output = Self;
  fn sub(self, rhs: Self) -> Self::Output {
    let x_diff = (self.0 | T::YZ_MASK) - (rhs.0 & T::X_MASK);
    let y_diff = (self.0 | T::XZ_MASK) - (rhs.0 & T::Y_MASK);
    let z_diff = (self.0 | T::XY_MASK) - (rhs.0 & T::Z_MASK);
    let diff = (x_diff & T::X_MASK) | (y_diff & T::Y_MASK) | (z_diff & T::Z_MASK);
    Self(diff & !(!T::ZERO << T::MAX_USABLE_BITS))
  }
}

impl<T: ZOrderStore> BitAnd<Self> for ZOrder<T> {
  type Output = Self;
  fn bitand(self, rhs: Self) -> Self::Output {
    Self(self.0 & rhs.0)
  }
}

impl<T: ZOrderStore> BitOr<Self> for ZOrder<T> {
  type Output = Self;
  fn bitor(self, rhs: Self) -> Self::Output {
    Self(self.0 | rhs.0)
  }
}

impl<T: ZOrderStore> BitXor<Self> for ZOrder<T> {
  type Output = Self;
  fn bitxor(self, rhs: Self) -> Self::Output {
    Self(self.0 ^ rhs.0)
  }
}

impl<T: ZOrderStore> Shl<usize> for ZOrder<T> {
  type Output = Self;
  fn shl(self, rhs: usize) -> Self {
    assert!(rhs < T::BITS_PER_ELEMENT);
    let mut result = self.0 << (rhs * 3);
    // Clear out any bits set beyond the usable bit range.
    result = result & !(!T::ZERO << T::MAX_USABLE_BITS);
    Self(result)
  }
}

impl<T: ZOrderStore> Shr<usize> for ZOrder<T> {
  type Output = Self;
  fn shr(self, rhs: usize) -> Self {
    assert!(rhs < T::BITS_PER_ELEMENT);
    let mut result = self.0 >> (rhs * 3);
    if T::SIGNED {
      let mut mask = (self.0 >> (T::MAX_USABLE_BITS - 3)) << (T::MAX_USABLE_BITS - (rhs * 3));
      for _ in 0..rhs {
        result = result | mask;
        mask = mask << 3;
      }
    }
    Self(result)
  }
}

impl<T: ZOrderStore> Debug for ZOrder<T>
where
  T::ElementType: Debug,
{
  fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
    let tuple = <Self as Into<(_, _, _)>>::into(*self);
    fmt.write_fmt(format_args!("ZOrder {:?}", tuple))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid_ranges() {
    macro_rules! assert_valid_ranges {
      ($T:ty, $bits_per_element:expr, $usable_bits:expr, $min_value:expr, $max_value:expr) => {
        assert_eq!(<$T as ZOrderStore>::BITS_PER_ELEMENT, $bits_per_element);
        assert_eq!(<$T as ZOrderStore>::MAX_USABLE_BITS, $usable_bits);
        assert_eq!(<$T as ZOrderStore>::ELEMENT_MIN, $min_value);
        assert_eq!(<$T as ZOrderStore>::ELEMENT_MAX, $max_value);
      };
    }

    assert_valid_ranges!(u16, 5, 15, 0, 31);
    assert_valid_ranges!(u32, 10, 30, 0, 1023);
    assert_valid_ranges!(u64, 21, 63, 0, 2097151);

    assert_valid_ranges!(i16, 5, 15, -16, 15);
    assert_valid_ranges!(i32, 10, 30, -512, 511);
    assert_valid_ranges!(i64, 21, 63, -1048576, 1048575);
  }

  #[test]
  fn decode_and_raw() {
    let o = ZOrder::<i16>::new(6, -2i8.pow(4), 15).unwrap();
    assert_eq!(o.into(), (6, -16, 15));

    //                    -16 + 8 + 4 + 2 + 1
    //                    -------------------
    //                x =   0   0   1   1   0
    //                y =  1   0   0   0   0
    //                z = 0   1   1   1   1
    assert_eq!(o.raw(), 0b010_100_101_101_100);

    let o = ZOrder::<u64>::new(0, 10, 2u32.pow(21) - 1).unwrap();
    assert_eq!(o.into(), (0, 10, 2u32.pow(21) - 1));
    assert_eq!(
      o.raw(),
      0b100_100_100_100_100_100_100_100_100_100_100_100_100_100_100_100_100_110_100_110_100
    );
  }

  #[test]
  fn out_of_range_returns_none() {
    assert_eq!(ZOrder::<u16>::new(0, 0, 2u8.pow(5)), None);
    assert_eq!(ZOrder::<u32>::new(0, 0, 2u16.pow(10)), None);
    assert_eq!(ZOrder::<u64>::new(0, 0, 2u32.pow(21)), None);

    assert_eq!(ZOrder::<i16>::new(0, 0, -2i8.pow(4) - 1), None);
    assert_eq!(ZOrder::<i32>::new(0, 0, -2i16.pow(9) - 1), None);
    assert_eq!(ZOrder::<i64>::new(0, 0, -2i32.pow(20) - 1), None);

    assert_eq!(ZOrder::<i16>::new(0, 0, 2i8.pow(4)), None);
    assert_eq!(ZOrder::<i32>::new(0, 0, 2i16.pow(9)), None);
    assert_eq!(ZOrder::<i64>::new(0, 0, 2i32.pow(20)), None);
  }

  #[test]
  fn bitwise_shifting() {
    let zero = ZOrder::<i32>::new(0, 0, 0).unwrap();
    assert_eq!(zero >> 3, zero << 0);

    let pos123 = ZOrder::<i32>::new(1, 2, 3).unwrap();
    assert_eq!(pos123 << 2, ZOrder::new(4, 8, 12).unwrap());

    let neg123 = ZOrder::<i32>::new(-1, -2, -3).unwrap();
    assert_eq!(neg123 << 2, ZOrder::new(-4, -8, -12).unwrap());
    assert_eq!(ZOrder::new(-4, -8, -12).unwrap() >> 2, neg123);
  }
}
