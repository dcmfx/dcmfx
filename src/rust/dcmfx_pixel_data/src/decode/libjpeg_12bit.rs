#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Returns the photometric interpretation used by data decoded using
/// libjpeg_12bit.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrFull422 => Ok(photometric_interpretation),

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{photometric_interpretation}' is not \
         supported"
      ),
    }),
  }
}

/// Decodes monochrome pixel data using libjpeg_12bit.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, PixelDataDecodeError> {
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
      BitsAllocated::Sixteen,
    ) => {
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
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG 12-bit monochrome decode not supported for photometric \
           interpretation '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

/// Decodes color pixel data using libjpeg_12bit.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrFull422,
      BitsAllocated::Sixteen,
    ) => {
      let color_space = match image_pixel_module.photometric_interpretation() {
        PhotometricInterpretation::YbrFull => ColorSpace::Ybr { is_422: false },
        PhotometricInterpretation::YbrFull422 => {
          ColorSpace::Ybr { is_422: true }
        }
        _ => ColorSpace::Rgb,
      };

      let pixels = decode(image_pixel_module, data)?;

      ColorImage::new_u16(
        image_pixel_module.columns(),
        image_pixel_module.rows(),
        pixels,
        color_space,
        image_pixel_module.bits_stored(),
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG 12-bit color decode not supported for photometric \
           interpretation '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<Vec<u16>, PixelDataDecodeError> {
  // Determine whether the output wil be in the YBR color space
  let is_ybr_color_space = image_pixel_module
    .photometric_interpretation()
    .is_ybr_full()
    || image_pixel_module
      .photometric_interpretation()
      .is_ybr_full_422();

  let mut error_message = [0 as core::ffi::c_char; 200];

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
      data.as_ptr() as *const core::ffi::c_void,
      data.len(),
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      u8::from(image_pixel_module.samples_per_pixel()).into(),
      is_ybr_color_space.into(),
      output_buffer.as_mut_ptr(),
      output_buffer.len(),
      error_message.as_mut_ptr(),
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error_c_str =
      unsafe { core::ffi::CStr::from_ptr(error_message.as_ptr()) };
    let error_str = error_c_str.to_str().unwrap_or("<invalid error>");

    return Err(PixelDataDecodeError::DataInvalid {
      details: format!("JPEG 12-bit decode failed with '{error_str}'"),
    });
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn libjpeg_12bit_decode(
      input_data: *const core::ffi::c_void,
      input_data_size: usize,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      is_ybr_color_space: usize,
      output_buffer: *mut u16,
      output_buffer_size: usize,
      error_message: *mut core::ffi::c_char,
    ) -> usize;
  }
}
