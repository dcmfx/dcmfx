#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::DataError;

use super::vec_cast;
use crate::{
  BitsAllocated, ColorImage, ColorSpace, PixelDataDefinition,
  SingleChannelImage,
};

/// Decodes single channel pixel data using CharLS.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(data, definition)?;

  let width = definition.columns();
  let height = definition.rows();

  if definition.bits_allocated() == BitsAllocated::Eight {
    SingleChannelImage::new_u8(width, height, pixels)
  } else if definition.bits_allocated() == BitsAllocated::Sixteen {
    SingleChannelImage::new_u16(width, height, unsafe {
      vec_cast::<u8, u16>(pixels)
    })
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

  let width = definition.columns();
  let height = definition.rows();

  if definition.bits_allocated() == BitsAllocated::Eight {
    ColorImage::new_u8(width, height, pixels, ColorSpace::RGB)
  } else if definition.bits_allocated() == BitsAllocated::Sixteen {
    let data = unsafe { vec_cast::<u8, u16>(pixels) };
    ColorImage::new_u16(width, height, data, ColorSpace::RGB)
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
  let width = definition.columns();
  let height = definition.rows();
  let samples_per_pixel = u8::from(definition.samples_per_pixel());
  let bits_allocated = u8::from(definition.bits_allocated());
  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  // Allocate output buffer
  let mut output_buffer = vec![
    0u8;
    definition.pixel_count()
      * usize::from(samples_per_pixel)
      * usize::from(bits_allocated / 8)
  ];

  let result = unsafe {
    ffi::charls_decode(
      data.as_ptr(),
      data.len() as u64,
      width.into(),
      height.into(),
      samples_per_pixel.into(),
      bits_allocated.into(),
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
      "JPEG LS pixel data decoding failed with '{error}'"
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
      error_buffer: *mut ::core::ffi::c_char,
      error_buffer_size: u32,
    ) -> i32;
  }
}
