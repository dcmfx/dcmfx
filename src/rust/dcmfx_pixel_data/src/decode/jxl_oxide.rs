#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec};

use jxl_oxide::{FrameBufferSample, JxlImage, Render, image::BitDepth};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Returns the photometric interpretation used by data decoded using jxl-oxide.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull422
    | PhotometricInterpretation::YbrRct
    | PhotometricInterpretation::Xyb => Ok(&PhotometricInterpretation::Rgb),

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{}' is not supported",
        photometric_interpretation
      ),
    }),
  }
}

/// Decodes monochrome pixel data using jxl-oxide.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, PixelDataDecodeError> {
  let (jxl_image, jxl_render) = decode(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
      BitDepth::IntegerSample { bits_per_sample: 8 },
    ) => {
      let mut buffer = vec![0u8; image_pixel_module.pixel_count()];
      render_samples(&jxl_render, &mut buffer)?;

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
      BitDepth::IntegerSample {
        bits_per_sample: 16,
      },
    ) => {
      let mut buffer = vec![0u16; image_pixel_module.pixel_count()];
      render_samples(&jxl_render, &mut buffer)?;

      MonochromeImage::new_u16(
        width,
        height,
        buffer,
        bits_stored,
        is_monochrome1,
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated, bit_depth) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG XL monochrome decode not supported for photometric \
           interpretation '{}', bits allocated '{}', input bit depth '{}'",
          photometric_interpretation,
          u8::from(bits_allocated),
          bit_depth.bits_per_sample()
        ),
      })
    }
  }
}

/// Decodes color pixel data using jxl-oxide.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  let (jxl_image, jxl_render) = decode(image_pixel_module, data)?;
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb,
      BitsAllocated::Eight,
      BitDepth::IntegerSample { bits_per_sample: 8 },
    ) => {
      let mut buffer = vec![0u8; image_pixel_module.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u8(width, height, buffer, ColorSpace::Rgb, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb
      | PhotometricInterpretation::YbrFull422
      | PhotometricInterpretation::YbrRct
      | PhotometricInterpretation::Xyb,
      BitsAllocated::Sixteen,
      BitDepth::IntegerSample {
        bits_per_sample: 16,
      },
    ) => {
      let mut buffer = vec![0u16; image_pixel_module.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u16(width, height, buffer, ColorSpace::Rgb, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated, bit_depth) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG XL color decode not supported for photometric \
           interpretation '{}', bits allocated '{}', input bit depth '{}'",
          photometric_interpretation,
          u8::from(bits_allocated),
          bit_depth.bits_per_sample()
        ),
      })
    }
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<(JxlImage, Render), PixelDataDecodeError> {
  if image_pixel_module.pixel_representation().is_signed() {
    return Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: "JPEG XL decode does not support signed pixel".to_string(),
    });
  }

  let mut image = JxlImage::read_with_defaults(data).map_err(|e| {
    PixelDataDecodeError::DataInvalid {
      details: format!("JPEG XL decode failed with '{}'", e),
    }
  })?;

  if image.width() != image_pixel_module.columns().into()
    || image.height() != image_pixel_module.rows().into()
  {
    return Err(PixelDataDecodeError::DataInvalid {
      details: "JPEG XL image has incorrect dimensions".to_string(),
    });
  }

  // Convert colors to sRGB
  if image_pixel_module.is_color() {
    image.request_color_encoding(jxl_oxide::EnumColourEncoding::srgb(
      jxl_oxide::RenderingIntent::default(),
    ));
  }

  let render =
    image
      .render_frame(0)
      .map_err(|e| PixelDataDecodeError::DataInvalid {
        details: format!("JPEG XL decode failed with '{}'", e),
      })?;

  Ok((image, render))
}

fn render_samples<Sample: FrameBufferSample>(
  jxl_render: &Render,
  buffer: &mut [Sample],
) -> Result<(), PixelDataDecodeError> {
  let sample_count = jxl_render.stream().write_to_buffer(buffer);

  if sample_count != buffer.len() {
    return Err(PixelDataDecodeError::DataInvalid {
      details: format!(
        "JPEG XL decode returned {} samples instead of {} samples",
        sample_count,
        buffer.len(),
      ),
    });
  }

  Ok(())
}
