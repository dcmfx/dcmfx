#[cfg(not(target_arch = "wasm32"))]
pub mod charls;
pub mod jpeg;
pub mod jpeg_decoder;
pub mod libjpeg_12bit;
pub mod native;
pub mod openjpeg;
pub mod rle_lossless;
pub mod ybr_to_rgb;

#[cfg(not(feature = "std"))]
mod no_std_allocator;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

/// Turns a `Vec<T>` into a Vec<U> using an unsafe cast.
///
unsafe fn vec_cast<T, U>(mut v: Vec<T>) -> Vec<U> {
  use core::mem::size_of;

  // The vector must contain a whole number of items of the target type
  assert_eq!((v.len() * size_of::<T>()) % size_of::<U>(), 0);

  let ptr = v.as_mut_ptr() as *mut U;
  let length = (v.len() * size_of::<T>()) / size_of::<U>();
  let capacity = (v.capacity() * size_of::<T>()) / size_of::<U>();

  ::core::mem::forget(v);

  unsafe { Vec::from_raw_parts(ptr, length, capacity) }
}
