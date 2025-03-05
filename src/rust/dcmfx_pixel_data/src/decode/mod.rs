#[cfg(not(target_arch = "wasm32"))]
pub mod charls;
pub mod jpeg;
#[cfg(not(target_arch = "wasm32"))]
pub mod jpeg2k;
pub mod jpeg_decoder;
pub mod native;
pub mod rle_lossless;
