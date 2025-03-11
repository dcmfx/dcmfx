#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use image::ImageBuffer;

use dcmfx_core::DataError;

use super::unsafe_vec_u8_into;
use crate::{
  BitsAllocated, ColorImage, PixelDataDefinition, SingleChannelImage,
};

/// Decodes single channel pixel data using CharLS.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(data, definition)?;

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if definition.bits_allocated == BitsAllocated::Eight
    && pixels.len() == pixel_count
  {
    Ok(SingleChannelImage::Uint8(
      ImageBuffer::from_raw(width, height, pixels).unwrap(),
    ))
  } else if definition.bits_allocated == BitsAllocated::Sixteen
    && pixels.len() == pixel_count * 2
  {
    Ok(SingleChannelImage::Uint16(
      ImageBuffer::from_raw(width, height, unsafe_vec_u8_into::<u16>(pixels))
        .unwrap(),
    ))
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not single channel".to_string(),
    ))
  }
}

/// Decodes color pixel data using CharLS.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(data, definition)?;

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if definition.bits_allocated == BitsAllocated::Eight
    && pixels.len() == pixel_count * 3
  {
    Ok(ColorImage::Uint8(
      ImageBuffer::from_raw(width, height, pixels).unwrap(),
    ))
  } else if definition.bits_allocated == BitsAllocated::Sixteen
    && pixels.len() == pixel_count * 6
  {
    let mut data = Vec::with_capacity(pixels.len() / 2);
    for chunk in pixels.chunks_exact(2) {
      data.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(ColorImage::Uint16(
      ImageBuffer::from_raw(width, height, data).unwrap(),
    ))
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not color".to_string(),
    ))
  }
}

fn decode(
  data: &[u8],
  definition: &PixelDataDefinition,
) -> Result<Vec<u8>, DataError> {
  let width = definition.columns as u32;
  let height = definition.rows as u32;
  let samples_per_pixel = usize::from(definition.samples_per_pixel) as u32;
  let bits_allocated = usize::from(definition.bits_allocated) as u32;
  let mut error_buffer = [0; 256];

  // Allocate output buffer
  let mut output_buffer = vec![
    0u8;
    definition.pixel_count()
      * samples_per_pixel as usize
      * (bits_allocated / 8) as usize
  ];

  let result = unsafe {
    ffi::charls_decode(
      data.as_ptr(),
      data.len() as u64,
      width,
      height,
      samples_per_pixel,
      bits_allocated,
      output_buffer.as_mut_ptr(),
      output_buffer.len() as u64,
      error_buffer.as_mut_ptr(),
      error_buffer.len() as u32,
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(DataError::new_value_invalid(format!(
      "CharLS decode failed, details: {error}"
    )));
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn charls_decode(
      input_data: *const u8,
      input_data_size: u64,
      width: u32,
      height: u32,
      samples_per_pixel: u32,
      bits_allocated: u32,
      output_data: *mut u8,
      output_data_size: u64,
      error_buffer: *mut i8,
      error_buffer_size: u32,
    ) -> i32;
  }
}
