#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, SingleChannelImage,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Decodes single channel pixel data using OpenJPEG.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();

  match (
    image_pixel_module.pixel_representation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PixelRepresentation::Unsigned,
      BitsAllocated::One | BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_u8(width, height, pixels)
    }

    (
      PixelRepresentation::Signed,
      BitsAllocated::One | BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_i8(width, height, pixels)
    }

    (PixelRepresentation::Unsigned, BitsAllocated::Sixteen) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_u16(width, height, pixels)
    }

    (PixelRepresentation::Signed, BitsAllocated::Sixteen) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_i16(width, height, pixels)
    }

    (PixelRepresentation::Unsigned, BitsAllocated::ThirtyTwo) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_u32(width, height, pixels)
    }

    (PixelRepresentation::Signed, BitsAllocated::ThirtyTwo) => {
      let pixels = decode(image_pixel_module, data)?;
      SingleChannelImage::new_i32(width, height, pixels)
    }
  }
}

/// Decodes color pixel data using OpenJPEG.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();

  match (
    &image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_palette8(width, height, pixels, palette.clone())
    }

    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_palette16(width, height, pixels, palette.clone())
    }

    (PhotometricInterpretation::PaletteColor { .. }, _) => {
      Err(DataError::new_value_invalid(format!(
        "OpenJPEG palette color data has invalid bits allocated '{}'",
        u8::from(image_pixel_module.bits_allocated())
      )))
    }

    (_, BitsAllocated::One | BitsAllocated::Eight) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u8(width, height, pixels, ColorSpace::RGB)
    }

    (_, BitsAllocated::Sixteen) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u16(width, height, pixels, ColorSpace::RGB)
    }

    (_, BitsAllocated::ThirtyTwo) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u32(width, height, pixels, ColorSpace::RGB)
    }
  }
}

fn decode<T: Clone + Default + bytemuck::Pod>(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<Vec<T>, DataError> {
  let samples_per_pixel = u8::from(image_pixel_module.samples_per_pixel());
  let bits_allocated = u8::from(image_pixel_module.bits_allocated()).max(8);
  let mut pixel_representation =
    u8::from(image_pixel_module.pixel_representation());
  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  // Allocate output buffer
  let mut output_buffer: Vec<T> = vec![
    T::default();
    image_pixel_module.pixel_count()
      * usize::from(samples_per_pixel)
  ];

  // Make FFI call into openjpeg to perform the decompression
  let result = unsafe {
    ffi::openjpeg_decode(
      data.as_ptr(),
      data.len() as u64,
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      samples_per_pixel.into(),
      bits_allocated.into(),
      &mut pixel_representation,
      output_buffer.as_mut_ptr() as *mut u8,
      (output_buffer.len() * core::mem::size_of::<T>()) as u64,
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
      "JPEG 2000 pixel data decoding failed with '{error}'"
    )));
  }

  if pixel_representation != u8::from(image_pixel_module.pixel_representation())
  {
    // If the data returned by OpenJPEG is unsigned, but signed data is expected
    // to be returned, then reinterpret it as signed two's complement integer
    // data
    if image_pixel_module.pixel_representation() == PixelRepresentation::Signed
    {
      convert_unsigned_values_to_signed_values(
        image_pixel_module,
        bytemuck::cast_slice_mut(&mut output_buffer),
      );
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
  image_pixel_module: &ImagePixelModule,
  data: &mut [u8],
) {
  match image_pixel_module.bits_allocated() {
    BitsAllocated::One => (),

    BitsAllocated::Eight => {
      let threshold = 2i16.pow(image_pixel_module.bits_stored() as u32 - 1);

      for i in data.iter_mut() {
        if *i as i16 >= threshold {
          *i = (*i as i16 - threshold * 2) as u8;
        }
      }
    }

    BitsAllocated::Sixteen => {
      let threshold = 2i32.pow(image_pixel_module.bits_stored() as u32 - 1);

      for chunk in data.chunks_exact_mut(2) {
        let value = u16::from_ne_bytes([chunk[0], chunk[1]]);
        if value as i32 >= threshold {
          chunk.copy_from_slice(
            &((value as i32 - threshold * 2) as i16).to_ne_bytes(),
          );
        }
      }
    }

    BitsAllocated::ThirtyTwo => {
      let threshold = 2i64.pow(image_pixel_module.bits_stored() as u32 - 1);

      for chunk in data.chunks_exact_mut(4) {
        let value =
          u32::from_ne_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        if i64::from(value) >= threshold {
          chunk.copy_from_slice(
            &((i64::from(value) - threshold * 2) as i32).to_ne_bytes(),
          );
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
      pixel_representation: *mut u8,
      output_data: *mut u8,
      output_data_size: u64,
      error_buffer: *mut ::core::ffi::c_char,
      error_buffer_size: u32,
    ) -> i32;
  }
}
