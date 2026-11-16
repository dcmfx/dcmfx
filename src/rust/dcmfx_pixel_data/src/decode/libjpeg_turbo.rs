#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Decodes monochrome 8-bit JPEG pixel data using libjpeg-turbo.
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
      BitsAllocated::Eight,
    ) => {
      let (pixels, _) =
        decode(image_pixel_module, data, OutputColorSpace::Grayscale)?;

      MonochromeImage::new_u8(
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
          "JPEG monochrome decode not supported for photometric interpretation \
           '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

/// Decodes color 8-bit JPEG pixel data using libjpeg-turbo.
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
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull422,
      BitsAllocated::Eight,
    ) => {
      let is_rgb = image_pixel_module.photometric_interpretation().is_rgb();

      let output_color_space = if is_rgb {
        OutputColorSpace::Rgb
      } else {
        OutputColorSpace::YCbCr
      };

      let (pixels, sampling_factors) =
        decode(image_pixel_module, data, output_color_space)?;

      let color_space = if is_rgb {
        ColorSpace::Rgb
      } else {
        // For YBR, look at the JPEG's sampling factors to determine whether YBR
        // 422 is in use. The photometric intepretation isn't always reliable.
        ColorSpace::Ybr {
          is_422: sampling_factors == (2, 1),
        }
      };

      ColorImage::new_u8(
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
          "JPEG color decode not supported for photometric interpretation \
           '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

/// The color space libjpeg-turbo is asked to output. These values must match
/// the `DCMFX_COLOR_SPACE_*` defines in `libjpeg_turbo_interface.c`.
///
#[derive(Clone, Copy)]
enum OutputColorSpace {
  Grayscale = 0,
  Rgb = 1,
  YCbCr = 2,
}

/// Decodes 8-bit JPEG data, returning the decoded pixels and the horizontal
/// and vertical sampling factors of the first component.
///
fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
  output_color_space: OutputColorSpace,
) -> Result<(Vec<u8>, (usize, usize)), PixelDataDecodeError> {
  let samples_per_pixel =
    usize::from(u8::from(image_pixel_module.samples_per_pixel()));

  let mut output_buffer =
    vec![0u8; image_pixel_module.pixel_count() * samples_per_pixel];

  let mut h_samp_factor = 0;
  let mut v_samp_factor = 0;
  let mut error_message = [0 as core::ffi::c_char; 200];

  let result = unsafe {
    ffi::libjpeg_turbo_decode_8bit(
      data.as_ptr(),
      data.len(),
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      samples_per_pixel,
      output_color_space as usize,
      output_buffer.as_mut_ptr(),
      output_buffer.len(),
      &mut h_samp_factor,
      &mut v_samp_factor,
      error_message.as_mut_ptr(),
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_message.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataDecodeError::DataInvalid {
      details: format!("JPEG decode failed with '{error}'"),
    });
  }

  Ok((output_buffer, (h_samp_factor, v_samp_factor)))
}

mod ffi {
  unsafe extern "C" {
    pub fn libjpeg_turbo_decode_8bit(
      input_data: *const u8,
      input_data_size: usize,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      output_color_space: usize,
      output_buffer: *mut u8,
      output_buffer_size: usize,
      h_samp_factor: *mut usize,
      v_samp_factor: *mut usize,
      error_message: *mut core::ffi::c_char,
    ) -> usize;
  }
}
