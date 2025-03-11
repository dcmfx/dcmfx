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

/// Turns a `Vec<u8>` to a Vec<T> using an unsafe cast.
///
fn unsafe_vec_u8_into<T>(mut v: Vec<u8>) -> Vec<T> {
  let ptr = v.as_mut_ptr() as *mut T;
  let len = v.len() / 2;
  let cap = v.capacity() / 2;

  ::core::mem::forget(v);

  unsafe { Vec::from_raw_parts(ptr, len, cap) }
}
