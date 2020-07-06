#![macro_use]

use {num_traits::PrimInt, std::mem::size_of};

const MASKS_8BIT: [u8; 2] = [0b_00000011, 0b_00001001];

const MASKS_16BIT: [u16; 4] = [
  0b_00000000_00011111,
  0b_00010000_00001111,
  0b_00010000_11000011,
  0b_00010010_01001001,
];

const MASKS_32BIT: [u32; 5] = [
  0b_00000000_00000000_00000011_11111111, // 0x3ff
  0b_00000011_00000000_00000000_11111111, // 0x30000ff
  0b_00000011_00000000_11110000_00001111, // 0x300f00f
  0b_00000011_00001100_00110000_11000011, // 0x30c30c3
  0b_00001001_00100100_10010010_01001001, // 0x9249249
];

const MASKS_64BIT: [u64; 6] = [
  0b_00000000_00000000_00000000_00000000_00000000_00011111_11111111_11111111, // 0x1fffff
  0b_00000000_00011111_00000000_00000000_00000000_00000000_11111111_11111111, // 0x1f00000000ffff
  0b_00000000_00011111_00000000_00000000_11111111_00000000_00000000_11111111, // 0x1f0000ff0000ff
  0b_00010000_00001111_00000000_11110000_00001111_00000000_11110000_00001111, // 0x100f00f00f00f00f
  0b_00010000_11000011_00001100_00110000_11000011_00001100_00110000_11000011, // 0x10c30c30c30c30c3
  0b_00010010_01001001_00100100_10010010_01001001_00100100_10010010_01001001, // 0x1249249249249249
];

pub trait ZOrderStore: PrimInt {
  type ElementType: PrimInt + Into<Self>;

  const BIT_SIZE: usize = size_of::<Self>() * 8;
  const MAX_USABLE_BITS: usize = Self::BITS_PER_ELEMENT * 3;
  const BITS_PER_ELEMENT: usize = Self::BIT_SIZE / 3;

  const SIGNED: bool;
  const ZERO: Self;
  const ONE: Self;

  const ELEMENT_BIT_SIZE: usize = size_of::<Self::ElementType>() * 8;
  const ELEMENT_MIN: Self::ElementType;
  const ELEMENT_MAX: Self::ElementType;

  const SIGN_SHIFT: usize = Self::ELEMENT_BIT_SIZE - Self::BITS_PER_ELEMENT;

  const X_MASK: Self;
  const Y_MASK: Self;
  const Z_MASK: Self;

  const XY_MASK: Self;
  const XZ_MASK: Self;
  const YZ_MASK: Self;

  fn split(x: Self::ElementType) -> Self;
  fn get(x: Self) -> Self::ElementType;
}

macro_rules! impl_store {
  ($UNSIGNED_TYPE: ty, $SIGNED_TYPE: ty, $UNSIGNED_ELEMENT_TYPE: ty, $SIGNED_ELEMENT_TYPE: ty, $X_MASK: expr, $var: ident, $SPLIT: block, $GET: block) => {
    impl ZOrderStore for $UNSIGNED_TYPE {
      type ElementType = $UNSIGNED_ELEMENT_TYPE;

      const SIGNED: bool = false;
      const ZERO: Self = 0;
      const ONE: Self = 1;

      const ELEMENT_MIN: Self::ElementType = 0;
      const ELEMENT_MAX: Self::ElementType = !(!0 << Self::BITS_PER_ELEMENT);

      const X_MASK: Self = $X_MASK as Self;
      const Y_MASK: Self = Self::X_MASK << 1;
      const Z_MASK: Self = Self::X_MASK << 2;

      const XY_MASK: Self = Self::X_MASK | Self::Y_MASK;
      const XZ_MASK: Self = Self::X_MASK | Self::Z_MASK;
      const YZ_MASK: Self = Self::Y_MASK | Self::Z_MASK;

      fn split(x: Self::ElementType) -> Self {
        let mut $var = x as Self;
        $SPLIT;
        $var
      }

      fn get(x: Self) -> Self::ElementType {
        let mut $var = x;
        $GET;
        $var as Self::ElementType
      }
    }

    impl ZOrderStore for $SIGNED_TYPE {
      type ElementType = $SIGNED_ELEMENT_TYPE;

      const SIGNED: bool = true;
      const ZERO: Self = 0;
      const ONE: Self = 1;

      const ELEMENT_MIN: Self::ElementType = !0 << (Self::BITS_PER_ELEMENT - 1);
      const ELEMENT_MAX: Self::ElementType = !Self::ELEMENT_MIN;

      const X_MASK: Self = $X_MASK as Self;
      const Y_MASK: Self = Self::X_MASK << 1;
      const Z_MASK: Self = Self::X_MASK << 2;

      const XY_MASK: Self = Self::X_MASK | Self::Y_MASK;
      const XZ_MASK: Self = Self::X_MASK | Self::Z_MASK;
      const YZ_MASK: Self = Self::Y_MASK | Self::Z_MASK;

      fn split(x: Self::ElementType) -> Self {
        let mut $var = x as $UNSIGNED_TYPE;
        $SPLIT;
        $var as Self
      }

      fn get(x: Self) -> Self::ElementType {
        let mut $var = x as $UNSIGNED_TYPE;
        $GET;
        $var as Self::ElementType
      }
    }
  };
}

impl_store!(
  u8,
  i8,
  u8,
  i8,
  MASKS_8BIT[MASKS_8BIT.len() - 1],
  x,
  {
    // x = x & MASKS_8BIT[0];
    x = (x | x << 2) & MASKS_8BIT[1];
  },
  {
    x = x & MASKS_8BIT[1];
    x = (x ^ (x >> 2)) & MASKS_8BIT[0];
  }
);

impl_store!(
  u16,
  i16,
  u8,
  i8,
  MASKS_16BIT[MASKS_16BIT.len() - 1],
  x,
  {
    // x = x & MASKS_16BIT[0];
    x = (x | x << 8) & MASKS_16BIT[1];
    x = (x | x << 4) & MASKS_16BIT[2];
    x = (x | x << 2) & MASKS_16BIT[3];
  },
  {
    x = x & MASKS_16BIT[3];
    x = (x ^ (x >> 2)) & MASKS_16BIT[2];
    x = (x ^ (x >> 4)) & MASKS_16BIT[1];
    x = (x ^ (x >> 8)) & MASKS_16BIT[0];
  }
);

impl_store!(
  u32,
  i32,
  u16,
  i16,
  MASKS_32BIT[MASKS_32BIT.len() - 1],
  x,
  {
    // x = x & MASKS_32BIT[0];
    x = (x | x << 16) & MASKS_32BIT[1];
    x = (x | x << 8) & MASKS_32BIT[2];
    x = (x | x << 4) & MASKS_32BIT[3];
    x = (x | x << 2) & MASKS_32BIT[4];
  },
  {
    x = x & MASKS_32BIT[4];
    x = (x ^ (x >> 2)) & MASKS_32BIT[3];
    x = (x ^ (x >> 4)) & MASKS_32BIT[2];
    x = (x ^ (x >> 8)) & MASKS_32BIT[1];
    x = (x ^ (x >> 16)) & MASKS_32BIT[0];
  }
);

impl_store!(
  u64,
  i64,
  u32,
  i32,
  MASKS_64BIT[MASKS_64BIT.len() - 1],
  x,
  {
    // x = x & MASKS_64BIT[0];
    x = (x | x << 32) & MASKS_64BIT[1];
    x = (x | x << 16) & MASKS_64BIT[2];
    x = (x | x << 8) & MASKS_64BIT[3];
    x = (x | x << 4) & MASKS_64BIT[4];
    x = (x | x << 2) & MASKS_64BIT[5];
  },
  {
    x = x & MASKS_64BIT[5];
    x = (x ^ (x >> 2)) & MASKS_64BIT[4];
    x = (x ^ (x >> 4)) & MASKS_64BIT[3];
    x = (x ^ (x >> 8)) & MASKS_64BIT[2];
    x = (x ^ (x >> 16)) & MASKS_64BIT[1];
    x = (x ^ (x >> 32)) & MASKS_64BIT[0];
  }
);
