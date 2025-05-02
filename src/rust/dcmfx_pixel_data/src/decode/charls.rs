#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
  },
};

/// Returns the photometric interpretation used by data decoded using CharLS.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => Ok(photometric_interpretation),

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

/// Decodes monochrome pixel data using CharLS.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  if image_pixel_module.bits_allocated() == BitsAllocated::Eight {
    let pixels = decode(data, image_pixel_module)?;
    MonochromeImage::new_u8(width, height, pixels, bits_stored, is_monochrome1)
  } else if image_pixel_module.bits_allocated() == BitsAllocated::Sixteen {
    let pixels = decode(data, image_pixel_module)?;
    MonochromeImage::new_u16(width, height, pixels, bits_stored, is_monochrome1)
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not monochrome".to_string(),
    ))
  }
}

/// Decodes color pixel data using CharLS.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  let color_space = if image_pixel_module.photometric_interpretation().is_ybr()
  {
    ColorSpace::Ybr
  } else {
    ColorSpace::Rgb
  };

  if image_pixel_module.bits_allocated() == BitsAllocated::Eight {
    let pixels = decode(data, image_pixel_module)?;
    ColorImage::new_u8(width, height, pixels, color_space, bits_stored)
  } else if image_pixel_module.bits_allocated() == BitsAllocated::Sixteen {
    let pixels = decode(data, image_pixel_module)?;
    ColorImage::new_u16(width, height, pixels, color_space, bits_stored)
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not color".to_string(),
    ))
  }
}

fn decode<T: Clone + Default>(
  data: &[u8],
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<T>, DataError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let samples_per_pixel = u8::from(image_pixel_module.samples_per_pixel());
  let bits_allocated = u8::from(image_pixel_module.bits_allocated());
  let mut error_buffer = [0 as ::core::ffi::c_char; 256];

  // Allocate output buffer
  let mut output_buffer = vec![
    T::default();
    image_pixel_module.pixel_count()
      * usize::from(samples_per_pixel)
  ];

  let result = unsafe {
    ffi::charls_decode(
      data.as_ptr(),
      data.len() as u64,
      width.into(),
      height.into(),
      samples_per_pixel.into(),
      bits_allocated.into(),
      output_buffer.as_mut_ptr() as *mut u8,
      (output_buffer.len() * core::mem::size_of::<T>()) as u64,
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
