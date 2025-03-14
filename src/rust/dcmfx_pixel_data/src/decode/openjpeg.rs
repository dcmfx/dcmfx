#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use image::ImageBuffer;

use dcmfx_core::DataError;

use super::vec_cast;
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
  let pixels = decode(definition, data)?;

  match (definition.pixel_representation, definition.bits_allocated) {
    (_, BitsAllocated::One) => unreachable!(),

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
          unsafe { vec_cast::<u8, i8>(pixels) },
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Unsigned, BitsAllocated::Sixteen) => {
      Ok(SingleChannelImage::Uint16(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe { vec_cast::<u8, u16>(pixels) },
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Signed, BitsAllocated::Sixteen) => {
      Ok(SingleChannelImage::Int16(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe { vec_cast::<u8, i16>(pixels) },
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Unsigned, BitsAllocated::ThirtyTwo) => {
      Ok(SingleChannelImage::Uint32(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe { vec_cast::<u8, u32>(pixels) },
        )
        .unwrap(),
      ))
    }

    (PixelRepresentation::Signed, BitsAllocated::ThirtyTwo) => {
      Ok(SingleChannelImage::Int32(
        ImageBuffer::from_raw(
          definition.columns as u32,
          definition.rows as u32,
          unsafe { vec_cast::<u8, i32>(pixels) },
        )
        .unwrap(),
      ))
    }
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
    BitsAllocated::One => unreachable!(),

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
        unsafe { vec_cast::<u8, u16>(pixels) },
      )
      .unwrap(),
    )),

    BitsAllocated::ThirtyTwo => Ok(ColorImage::Uint32(
      ImageBuffer::from_raw(
        definition.columns as u32,
        definition.rows as u32,
        unsafe { vec_cast::<u8, u32>(pixels) },
      )
      .unwrap(),
    )),
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<Vec<u8>, DataError> {
  if definition.bits_allocated == BitsAllocated::One {
    return Err(DataError::new_value_invalid(
      "OpenJPEG does not support 1-bit pixel data".to_string(),
    ));
  }

  let width = definition.columns as u32;
  let height = definition.rows as u32;
  let samples_per_pixel = usize::from(definition.samples_per_pixel) as u32;
  let bits_allocated = usize::from(definition.bits_allocated) as u32;
  let mut pixel_representation =
    usize::from(definition.pixel_representation) as u32;
  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

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
      &mut pixel_representation,
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

  if pixel_representation != usize::from(definition.pixel_representation) as u32
  {
    // If the data returned by OpenJPEG is unsigned, but signed data is expected
    // to be returned, then reinterpret it as signed two's complement integer
    // data
    if definition.pixel_representation == PixelRepresentation::Signed {
      convert_unsigned_values_to_signed_values(definition, &mut output_buffer);
    } else {
      return Err(DataError::new_value_invalid(
        "OpenJPEG decode returned signed data but the pixel representation \
         specifies unsigned data"
          .to_string(),
      ));
    }
  }

  Ok(output_buffer)
}

/// Converts unsigned values to signed two's complement values based on the
/// number of bits stored in each value.
///
fn convert_unsigned_values_to_signed_values(
  definition: &PixelDataDefinition,
  data: &mut [u8],
) {
  match definition.bits_allocated {
    BitsAllocated::One => unreachable!(),

    BitsAllocated::Eight => {
      let threshold = 2i16.pow(definition.bits_stored as u32 - 1);

      for i in data.iter_mut() {
        if *i as i16 >= threshold {
          let value = (*i as i16 - threshold * 2) as i8;
          *i = value.to_le_bytes()[0];
        }
      }
    }

    BitsAllocated::Sixteen => {
      let threshold = 2i32.pow(definition.bits_stored as u32 - 1);

      for chunk in data.chunks_exact_mut(2) {
        let value = u16::from_ne_bytes([chunk[0], chunk[1]]);
        if value as i32 >= threshold {
          let bytes = ((value as i32 - threshold * 2) as i16).to_ne_bytes();
          chunk[0] = bytes[0];
          chunk[1] = bytes[1];
        }
      }
    }

    BitsAllocated::ThirtyTwo => {
      let threshold = 2i64.pow(definition.bits_stored as u32 - 1);

      for chunk in data.chunks_exact_mut(4) {
        let value =
          u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if value as i64 >= threshold {
          let bytes = ((value as i64 - threshold * 2) as i32).to_ne_bytes();
          chunk[0] = bytes[0];
          chunk[1] = bytes[1];
          chunk[2] = bytes[2];
          chunk[3] = bytes[3];
        }
      }
    }
  }
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
      pixel_representation: *mut u32,
      output_data: *mut u8,
      output_data_size: u64,
      error_buffer: *mut ::core::ffi::c_char,
      error_buffer_size: u32,
    ) -> i32;
  }
}
