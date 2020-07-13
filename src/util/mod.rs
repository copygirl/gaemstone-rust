use {num_traits::PrimInt, std::mem::size_of};

pub use {chunked_octree::*, palette_store::*, z_order::*, z_order_store::*};

mod chunked_octree;
mod palette_store;
mod z_order;
mod z_order_store;

pub fn integer_log2<T: PrimInt>(value: T) -> u32 {
  // Logarithm of a non-positive integer is undefined.
  assert!(value > T::zero());
  // Would make this a constant expression if I could, but changing `let` to `const`
  // results in error E0401: "can't use generic parameters from outer function".
  let bitsize_minus_one = size_of::<T>() as u32 * 8 - 1;
  bitsize_minus_one - value.leading_zeros()
}
