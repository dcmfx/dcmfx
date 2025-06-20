#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Decodes monochrome pixel data using libjxl.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, PixelDataDecodeError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    ) => {
      let buffer = decode::<u8>(image_pixel_module, data)?;

      MonochromeImage::new_u8(
        width,
        height,
        buffer,
        bits_stored,
        is_monochrome1,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Sixteen,
    ) => {
      let buffer = decode::<u16>(image_pixel_module, data)?;

      MonochromeImage::new_u16(
        width,
        height,
        buffer,
        bits_stored,
        is_monochrome1,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG XL monochrome decode with libjxl not supported for photometric \
           interpretation '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated),
        ),
      })
    }
  }
}

/// Decodes color pixel data using libjxl.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb,
      BitsAllocated::Eight,
    ) => {
      let buffer = decode::<u8>(image_pixel_module, data)?;

      ColorImage::new_u8(width, height, buffer, ColorSpace::Rgb, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb,
      BitsAllocated::Sixteen,
    ) => {
      let buffer = decode::<u16>(image_pixel_module, data)?;

      ColorImage::new_u16(width, height, buffer, ColorSpace::Rgb, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG XL color decode with libjxl not supported for photometric \
           interpretation '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated),
        ),
      })
    }
  }
}

fn decode<T: Clone + Default>(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<Vec<T>, PixelDataDecodeError> {
  let mut error_message = [0 as core::ffi::c_char; 200];

  // Allocate output buffer
  let mut output_buffer =
    vec![
      T::default();
      image_pixel_module.pixel_count()
        * usize::from(u8::from(image_pixel_module.samples_per_pixel()))
    ];

  // Make FFI call into libjxl to perform the decompression
  let result = unsafe {
    ffi::libjxl_decode(
      data.as_ptr() as *const core::ffi::c_void,
      data.len(),
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      u8::from(image_pixel_module.bits_allocated()).into(),
      output_buffer.as_mut_ptr() as *mut core::ffi::c_void,
      output_buffer.len()
        * usize::from(u8::from(image_pixel_module.bits_allocated()) / 8),
      error_message.as_mut_ptr(),
      error_message.len(),
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error_c_str =
      unsafe { core::ffi::CStr::from_ptr(error_message.as_ptr()) };
    let error_str = error_c_str.to_str().unwrap_or("<invalid error>");

    return Err(PixelDataDecodeError::DataInvalid {
      details: format!("libjxl decode failed with '{error_str}'"),
    });
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn libjxl_decode(
      input_data: *const core::ffi::c_void,
      input_data_size: usize,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      bits_allocated: usize,
      output_buffer: *mut core::ffi::c_void,
      output_buffer_size: usize,
      error_buffer: *mut core::ffi::c_char,
      error_buffer_size: usize,
    ) -> usize;
  }
}
