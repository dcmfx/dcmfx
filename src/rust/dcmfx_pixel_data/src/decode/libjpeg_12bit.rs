#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
  },
};
use dcmfx_core::DataError;

/// Returns the photometric interpretation used by data decoded using
/// libjpeg_12bit.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull => Ok(&PhotometricInterpretation::Rgb),

    _ => {
      Err(PixelDataDecodeError::NotSupported {
        details: format!(
          "Decoding photometric interpretation '{}' is not supported",
          photometric_interpretation
        ),
      })
    }
  }
}

/// Decodes monochrome pixel data using libjpeg_12bit.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, DataError> {
  let pixels = decode(image_pixel_module, data)?;
  MonochromeImage::new_u16(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    image_pixel_module.bits_stored(),
    image_pixel_module
      .photometric_interpretation()
      .is_monochrome1(),
  )
}

/// Decodes color pixel data using libjpeg_12bit.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(image_pixel_module, data)?;

  let color_space = if image_pixel_module.photometric_interpretation().is_ybr()
  {
    ColorSpace::Ybr
  } else {
    ColorSpace::Rgb
  };

  ColorImage::new_u16(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    color_space,
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
      u32::from(image_pixel_module.photometric_interpretation().is_ybr()),
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
      samples_per_pixel: u32,
      is_ybr_color_space: u32,
      output_buffer: *mut u16,
      output_buffer_size: u64,
      error_message: *mut ::core::ffi::c_char,
    ) -> i32;
  }
}
