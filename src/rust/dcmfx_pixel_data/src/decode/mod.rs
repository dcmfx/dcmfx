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
