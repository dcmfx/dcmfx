#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  BitsAllocated, ColorImage, ColorSpace, PixelDataDefinition,
  SingleChannelImage,
};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using libjpeg_12bit.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(definition, data)?;
  SingleChannelImage::new_u16(definition.columns, definition.rows, pixels)
}

/// Decodes color pixel data using libjpeg_12bit.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(definition, data)?;
  ColorImage::new_u16(
    definition.columns,
    definition.rows,
    pixels,
    ColorSpace::RGB,
  )
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<Vec<u16>, DataError> {
  if definition.bits_allocated != BitsAllocated::Sixteen {
    return Err(DataError::new_value_invalid(format!(
      "JPEG 12-bit pixel data must have 16 bits allocated but has {}",
      usize::from(definition.bits_allocated)
    )));
  }

  let mut error_message = [0 as ::core::ffi::c_char; 200];

  // Allocate output buffer
  let mut output_buffer = vec![
    0u16;
    definition.pixel_count()
      * usize::from(definition.samples_per_pixel)
  ];

  // Make FFI call into libjpeg_12bit to perform the decompression
  let result = unsafe {
    ffi::libjpeg_12bit_decode(
      data.as_ptr(),
      data.len() as u64,
      definition.columns as u32,
      definition.rows as u32,
      usize::from(definition.samples_per_pixel) as u32,
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
