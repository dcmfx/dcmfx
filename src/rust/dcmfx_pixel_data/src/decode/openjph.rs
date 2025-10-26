#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Decodes monochrome pixel data using OpenJPH.
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
      let pixels = decode(image_pixel_module, data)?;
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
        pixel_representation: PixelRepresentation::Signed,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      MonochromeImage::new_i8(
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
      let pixels = decode(image_pixel_module, data)?;
      MonochromeImage::new_u16(
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
        pixel_representation: PixelRepresentation::Signed,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      MonochromeImage::new_i16(
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
      BitsAllocated::ThirtyTwo,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      MonochromeImage::new_u32(
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
        pixel_representation: PixelRepresentation::Signed,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      BitsAllocated::ThirtyTwo,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      MonochromeImage::new_i32(
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
          "OpenJPH monochrome decode not supported with photometric \
           interpretation '{}' and bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated),
        ),
      })
    }
  }
}

/// Decodes color pixel data using OpenJPH.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  let color_space = if image_pixel_module.photometric_interpretation()
    == &PhotometricInterpretation::YbrFull
  {
    ColorSpace::Ybr { is_422: false }
  } else {
    ColorSpace::Rgb
  };

  match (
    &image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
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
      PhotometricInterpretation::PaletteColor { palette },
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_palette16(
        width,
        height,
        pixels,
        palette.clone(),
        bits_stored,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct,
      BitsAllocated::Eight,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u8(width, height, pixels, color_space, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct,
      BitsAllocated::Sixteen,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u16(width, height, pixels, color_space, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull
      | PhotometricInterpretation::YbrIct
      | PhotometricInterpretation::YbrRct,
      BitsAllocated::ThirtyTwo,
    ) => {
      let pixels = decode(image_pixel_module, data)?;
      ColorImage::new_u32(width, height, pixels, color_space, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "OpenJPH color decode not supported with photometric interpretation \
           '{}' and bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

fn decode<T: Clone + Default + bytemuck::Pod>(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<Vec<T>, PixelDataDecodeError> {
  let samples_per_pixel = u8::from(image_pixel_module.samples_per_pixel());
  let bits_allocated = u8::from(image_pixel_module.bits_allocated()).max(8);
  let bits_stored = image_pixel_module.bits_stored();
  let pixel_representation =
    u8::from(image_pixel_module.pixel_representation()) as usize;
  let mut error_buffer = [0 as core::ffi::c_char; 256];

  // Allocate output buffer
  let mut output_buffer: Vec<T> = vec![
    T::default();
    image_pixel_module.pixel_count()
      * usize::from(samples_per_pixel)
  ];

  // Make FFI call into OpenJPH to perform the decompression
  let result = unsafe {
    ffi::openjph_decode(
      data.as_ptr() as *const core::ffi::c_void,
      data.len(),
      image_pixel_module.columns().into(),
      image_pixel_module.rows().into(),
      samples_per_pixel.into(),
      bits_allocated.into(),
      bits_stored.into(),
      pixel_representation,
      output_buffer.as_mut_ptr() as *mut core::ffi::c_void,
      output_buffer.len() * core::mem::size_of::<T>(),
      error_buffer.as_mut_ptr(),
      error_buffer.len(),
    )
  };

  // On error, read the error message string
  if result != 0 {
    let error = unsafe { core::ffi::CStr::from_ptr(error_buffer.as_ptr()) }
      .to_str()
      .unwrap_or("<invalid error>");

    return Err(PixelDataDecodeError::DataInvalid {
      details: format!("OpenJPH decode failed with '{error}'"),
    });
  }

  Ok(output_buffer)
}

mod ffi {
  unsafe extern "C" {
    pub fn openjph_decode(
      input_data: *const core::ffi::c_void,
      input_data_size: usize,
      width: usize,
      height: usize,
      samples_per_pixel: usize,
      bits_allocated: usize,
      bits_stored: usize,
      pixel_representation: usize,
      output_data: *mut core::ffi::c_void,
      output_data_size: usize,
      error_buffer: *mut core::ffi::c_char,
      error_buffer_size: usize,
    ) -> usize;
  }
}
