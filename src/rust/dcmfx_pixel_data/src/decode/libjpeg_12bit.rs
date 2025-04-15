#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, SingleChannelImage,
  iods::image_pixel_module::{BitsAllocated, ImagePixelModule},
};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using libjpeg_12bit.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(image_pixel_module, data)?;
  SingleChannelImage::new_u16(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    image_pixel_module.bits_stored(),
  )
}

/// Decodes color pixel data using libjpeg_12bit.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(image_pixel_module, data)?;
  ColorImage::new_u16(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    ColorSpace::RGB,
    image_pixel_module.bits_stored(),
  )
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<Vec<u16>, DataError> {
  if image_pixel_module.bits_allocated() != BitsAllocated::Sixteen {
    return Err(DataError::new_value_invalid(format!(
      "JPEG 12-bit pixel data must have 16 bits allocated but has {}",
      u8::from(image_pixel_module.bits_allocated())
    )));
  }

  let mut error_message = [0 as ::core::ffi::c_char; 200];

  // Allocate output buffer
  let mut output_buffer =
    vec![
      0u16;
      image_pixel_module.pixel_count()
        * usize::from(u8::from(image_pixel_module.samples_per_pixel()))
    ];

  // Make FFI call into libjpeg_12bit to perform the decompression
  let result = unsafe {
    ffi::libjpeg_12bit_decode(
      data.as_ptr(),
      data.len() as u64,
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      output_buffer.as_mut_ptr(),
      output_buffer.len() as u64,
      error_message.as_mut_ptr(),
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error_c_str =
      unsafe { core::ffi::CStr::from_ptr(error_message.as_ptr()) };
    let error_str = error_c_str.to_str().unwrap_or("<invalid error>");

    return Err(DataError::new_value_invalid(format!(
      "JPEG 12-bit pixel data decoding failed with '{error_str}'"
    )));
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn libjpeg_12bit_decode(
      jpeg_data: *const u8,
      jpeg_size: u64,
      width: u32,
      height: u32,
      channels: u32,
      output_buffer: *mut u16,
      output_buffer_size: u64,
      error_message: *mut ::core::ffi::c_char,
    ) -> i32;
  }
}
