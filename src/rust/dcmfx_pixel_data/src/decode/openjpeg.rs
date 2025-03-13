#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use image::ImageBuffer;

use dcmfx_core::DataError;

use super::unsafe_vec_u8_into;
use crate::{
  BitsAllocated, ColorImage, PixelDataDefinition, PixelRepresentation,
  SingleChannelImage,
};

/// Decodes single channel pixel data using OpenJPEG.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  if !definition.is_grayscale()
    || definition.bits_allocated == BitsAllocated::One
  {
    return Err(DataError::new_value_invalid(
      "OpenJPEG pixel data is not single channel".to_string(),
    ));
  }

  let pixels = decode(definition, data)?;

  match (definition.pixel_representation, definition.bits_allocated) {
    (PixelRepresentation::Unsigned, BitsAllocated::Eight) => {
      Ok(SingleChannelImage::Uint8(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          pixels,
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Signed, BitsAllocated::Eight) => {
      Ok(SingleChannelImage::Int8(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe_vec_u8_into::<i8>(pixels),
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Unsigned, BitsAllocated::Sixteen) => {
      Ok(SingleChannelImage::Uint16(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe_vec_u8_into::<u16>(pixels),
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Signed, BitsAllocated::Sixteen) => {
      Ok(SingleChannelImage::Int16(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe_vec_u8_into::<i16>(pixels),
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Unsigned, BitsAllocated::ThirtyTwo) => {
      Ok(SingleChannelImage::Uint32(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe_vec_u8_into::<u32>(pixels),
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Signed, BitsAllocated::ThirtyTwo) => {
      Ok(SingleChannelImage::Int32(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe_vec_u8_into::<i32>(pixels),
        )
        .unwrap(),
      ))
    }

    _ => Err(DataError::new_value_invalid(
      "JPEG 2000 pixel data is not single channel".to_string(),
    )),
  }
}

/// Decodes color pixel data using OpenJPEG.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(definition, data)?;

  match definition.bits_allocated {
    BitsAllocated::Eight => Ok(ColorImage::Uint8(
      ImageBuffer::from_raw(
        definition.columns as u32,
        definition.rows as u32,
        pixels,
      )
      .unwrap(),
    )),

    BitsAllocated::Sixteen => Ok(ColorImage::Uint16(
      ImageBuffer::from_raw(
        definition.columns as u32,
        definition.rows as u32,
        unsafe_vec_u8_into::<u16>(pixels),
      )
      .unwrap(),
    )),

    BitsAllocated::ThirtyTwo => Ok(ColorImage::Uint32(
      ImageBuffer::from_raw(
        definition.columns as u32,
        definition.rows as u32,
        unsafe_vec_u8_into::<u32>(pixels),
      )
      .unwrap(),
    )),

    _ => Err(DataError::new_value_invalid(
      "JPEG 2000 pixel data is not RGB".to_string(),
    )),
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<Vec<u8>, DataError> {
  let width = definition.columns as u32;
  let height = definition.rows as u32;
  let samples_per_pixel = usize::from(definition.samples_per_pixel) as u32;
  let bits_allocated = usize::from(definition.bits_allocated) as u32;
  let pixel_representation = if definition.pixel_representation.is_signed() {
    1
  } else {
    0
  };
  let mut error_buffer = [0; 256];

  // Allocate output buffer
  let mut output_buffer = vec![
    0u8;
    definition.pixel_count()
      * samples_per_pixel as usize
      * (bits_allocated / 8) as usize
  ];

  // Make FFI call into openjpeg to perform the decompression
  let result = unsafe {
    ffi::openjpeg_decode(
      data.as_ptr(),
      data.len() as u64,
      width,
      height,
      samples_per_pixel,
      bits_allocated,
      pixel_representation,
      output_buffer.as_mut_ptr(),
      output_buffer.len() as u64,
      error_buffer.as_mut_ptr(),
      error_buffer.len() as u32,
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(DataError::new_value_invalid(format!(
      "OpenJPEG decode failed with '{error}'"
    )));
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn openjpeg_decode(
      input_data: *const u8,
      input_data_size: u64,
      width: u32,
      height: u32,
      samples_per_pixel: u32,
      bits_allocated: u32,
      pixel_representation: u32,
      output_data: *mut u8,
      output_data_size: u64,
      error_buffer: *mut i8,
      error_buffer_size: u32,
    ) -> i32;
  }
}
