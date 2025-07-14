#[cfg(not(feature = "std"))]
use alloc::{format, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Returns the photometric interpretation used by data decoded using CharLS.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::PaletteColor { .. } => {
      Ok(photometric_interpretation)
    }

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{photometric_interpretation}' is not \
         supported"
      ),
    }),
  }
}

/// Decodes monochrome pixel data using CharLS.
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
      let pixels = decode(data, image_pixel_module)?;
      MonochromeImage::new_u8(
        width,
        height,
        pixels,
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
      let pixels = decode(data, image_pixel_module)?;
      MonochromeImage::new_u16(
        width,
        height,
        pixels,
        bits_stored,
        is_monochrome1,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG-LS monochrome decode not supported for photometric \
           interpretation '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

/// Decodes color pixel data using CharLS.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  let color_space = if image_pixel_module.photometric_interpretation().is_rgb()
  {
    ColorSpace::Rgb
  } else {
    ColorSpace::Ybr { is_422: false }
  };

  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(data, image_pixel_module)?;
      ColorImage::new_u8(width, height, pixels, color_space, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(data, image_pixel_module)?;
      ColorImage::new_palette8(
        width,
        height,
        pixels,
        palette.clone(),
        bits_stored,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(data, image_pixel_module)?;
      ColorImage::new_u16(width, height, pixels, color_space, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(data, image_pixel_module)?;
      ColorImage::new_palette16(
        width,
        height,
        pixels,
        palette.clone(),
        bits_stored,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG-LS color decode not supported for photometric interpretation \
           '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

fn decode<T: Clone + Default>(
  data: &[u8],
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<T>, PixelDataDecodeError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let samples_per_pixel = u8::from(image_pixel_module.samples_per_pixel());
  let bits_allocated = u8::from(image_pixel_module.bits_allocated());
  let mut error_buffer = [0 as core::ffi::c_char; 256];

  // Allocate output buffer
  let mut output_buffer = vec![
    T::default();
    image_pixel_module.pixel_count()
      * usize::from(samples_per_pixel)
  ];

  let result = unsafe {
    ffi::charls_decode(
      data.as_ptr() as *mut core::ffi::c_void,
      data.len(),
      width.into(),
      height.into(),
      samples_per_pixel.into(),
      bits_allocated.into(),
      output_buffer.as_mut_ptr() as *mut core::ffi::c_void,
      output_buffer.len() * core::mem::size_of::<T>(),
      error_buffer.as_mut_ptr(),
      error_buffer.len(),
    )
  };

  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataDecodeError::DataInvalid {
      details: format!("JPEG-LS pixel data decode failed with '{error}'"),
    });
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn charls_decode(
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
