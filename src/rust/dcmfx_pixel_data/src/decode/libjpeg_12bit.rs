use image::ImageBuffer;

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
    Ok(SingleChannelImage::Uint16(
      ImageBuffer::from_raw(width, height, pixel_data).unwrap(),
    ))
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
    Ok(ColorImage::Uint16(
      ImageBuffer::from_raw(width, height, pixel_data).unwrap(),
    ))
  } else {
    Err(DataError::new_value_invalid(
      "JPEG 12-bit pixel data is not color".to_string(),
    ))
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(u32, u32, usize, Vec<u16>), DataError> {
  let mut width: libc::c_int = 0;
  let mut height: libc::c_int = 0;
  let mut channels: libc::c_int = 0;
  let mut error_message: [libc::c_char; 200] = [0; 200];

  // Allocate output buffer
  let mut output_buffer = vec![
    0u16;
    definition.pixel_count()
      * usize::from(definition.samples_per_pixel)
  ];

  // Make FFI call into libjpeg_12bit to perform the decompression
  let result = unsafe {
    ffi::ijg_decode_jpeg_12bit(
      data.as_ptr(),
      data.len() as libc::size_t,
      &mut width,
      &mut height,
      &mut channels,
      output_buffer.as_mut_ptr(),
      output_buffer.len() as libc::size_t,
      error_message.as_mut_ptr(),
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error_c_str =
      unsafe { std::ffi::CStr::from_ptr(error_message.as_ptr()) };
    let error_str = error_c_str.to_str().unwrap_or("<invalid error>");

    return Err(DataError::new_value_invalid(format!(
      "JPEG 12-bit pixel data decode failed, details: {error_str}"
    )));
  }

  if width != definition.columns.into() || height != definition.rows.into() {
    return Err(DataError::new_value_invalid(
      "JPEG 12-bit pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok((
    width as u32,
    height as u32,
    channels as usize,
    output_buffer,
  ))
}

mod ffi {
  unsafe extern "C" {
    pub fn ijg_decode_jpeg_12bit(
      jpeg_data: *const u8,
      jpeg_size: libc::size_t,
      width: *mut libc::c_int,
      height: *mut libc::c_int,
      channels: *mut libc::c_int,
      output_buffer: *mut u16,
      output_buffer_size: libc::size_t,
      error_message: *mut libc::c_char,
    ) -> libc::c_int;
  }
}
