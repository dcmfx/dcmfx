#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using libjpeg_12bit.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (width, height, channels, pixel_data) = decode(definition, data)?;

  if channels == 1 && pixel_data.len() == definition.pixel_count() {
    Ok(SingleChannelImage::new_u16(width, height, pixel_data).unwrap())
  } else {
    Err(DataError::new_value_invalid(
      "JPEG Extended pixel data is not single channel".to_string(),
    ))
  }
}

/// Decodes color pixel data using libjpeg_12bit.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let (width, height, channels, pixel_data) = decode(definition, data)?;

  if channels == 3 && pixel_data.len() == definition.pixel_count() * 3 {
    Ok(ColorImage::new_u16(width, height, pixel_data).unwrap())
  } else {
    Err(DataError::new_value_invalid(
      "JPEG 12-bit pixel data is not color".to_string(),
    ))
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(u16, u16, usize, Vec<u16>), DataError> {
  let mut width: u32 = 0;
  let mut height: u32 = 0;
  let mut channels: u32 = 0;
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
      &mut width,
      &mut height,
      &mut channels,
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
      "JPEG 12-bit decode failed with '{error_str}'"
    )));
  }

  if width != definition.columns.into() || height != definition.rows.into() {
    return Err(DataError::new_value_invalid(
      "JPEG 12-bit pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok((
    width as u16,
    height as u16,
    channels as usize,
    output_buffer,
  ))
}

mod ffi {
  unsafe extern "C" {
    pub fn libjpeg_12bit_decode(
      jpeg_data: *const u8,
      jpeg_size: u64,
      width: *mut u32,
      height: *mut u32,
      channels: *mut u32,
      output_buffer: *mut u16,
      output_buffer_size: u64,
      error_message: *mut ::core::ffi::c_char,
    ) -> i32;
  }
}
