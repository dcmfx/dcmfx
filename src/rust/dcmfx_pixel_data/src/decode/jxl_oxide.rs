#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec};

use jxl_oxide::{FrameBufferSample, JxlImage, Render, image::BitDepth};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
  },
};

/// Returns the photometric interpretation used by data decoded using jxl-oxide.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull422
    | PhotometricInterpretation::YbrIct
    | PhotometricInterpretation::YbrRct
    | PhotometricInterpretation::Xyb => Ok(&PhotometricInterpretation::Rgb),

    _ => Err(PixelDataDecodeError::NotSupported {
      details: format!(
        "Decoding photometric interpretation '{}' is not supported",
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
) -> Result<MonochromeImage, DataError> {
  let (jxl_image, jxl_render) = decode(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  match (
    image_pixel_module.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (BitsAllocated::Eight, BitDepth::IntegerSample { bits_per_sample: 8 }) => {
      let mut buffer = vec![0u8; image_pixel_module.pixel_count()];
      render_samples(&jxl_render, &mut buffer)?;

      MonochromeImage::new_u8(
        width,
        height,
        buffer,
        bits_stored,
        is_monochrome1,
      )
    }

    (
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
    }

    _ => Err(DataError::new_value_invalid(
      "JPEG XL pixel data does not contain a supported monochrome image"
        .to_string(),
    )),
  }
}

/// Decodes color pixel data using jxl-oxide.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  if image_pixel_module.bits_allocated() == BitsAllocated::One {
    return Err(DataError::new_value_invalid(
      "JPEG XL does not support 1-bit pixel data".to_string(),
    ));
  }

  let (jxl_image, jxl_render) = decode(image_pixel_module, data)?;
  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  match (
    image_pixel_module.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (BitsAllocated::Eight, BitDepth::IntegerSample { bits_per_sample: 8 }) => {
      let mut buffer = vec![0u8; image_pixel_module.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u8(width, height, buffer, ColorSpace::Rgb, bits_stored)
    }

    (
      BitsAllocated::Sixteen,
      BitDepth::IntegerSample {
        bits_per_sample: 16,
      },
    ) => {
      let mut buffer = vec![0u16; image_pixel_module.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u16(width, height, buffer, ColorSpace::Rgb, bits_stored)
    }

    _ => Err(DataError::new_value_invalid(
      "JPEG XL pixel data does not contain a supported color image".to_string(),
    )),
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<(JxlImage, Render), DataError> {
  if image_pixel_module.pixel_representation().is_signed() {
    return Err(DataError::new_value_invalid(
      "JPEG XL decoding of signed pixel data is not supported".to_string(),
    ));
  }

  let mut image = JxlImage::read_with_defaults(data).map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG XL pixel data decoding failed with '{}'",
      e
    ))
  })?;

  if image.width() != image_pixel_module.columns().into()
    || image.height() != image_pixel_module.rows().into()
  {
    return Err(DataError::new_value_invalid(
      "JPEG XL pixel data has incorrect dimensions".to_string(),
    ));
  }

  // Convert colors to sRGB
  if image_pixel_module.is_color() {
    image.request_color_encoding(jxl_oxide::EnumColourEncoding::srgb_gamma22(
      jxl_oxide::RenderingIntent::default(),
    ));
  }

  let render = image.render_frame(0).map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG XL pixel data decoding failed with '{}'",
      e
    ))
  })?;

  Ok((image, render))
}

fn render_samples<Sample: FrameBufferSample>(
  jxl_render: &Render,
  buffer: &mut [Sample],
) -> Result<(), DataError> {
  let sample_count = jxl_render.stream().write_to_buffer(buffer);

  if sample_count != buffer.len() {
    return Err(DataError::new_value_invalid(format!(
      "JPEG XL pixel data decoding returned {} samples instead of {} samples",
      sample_count,
      buffer.len(),
    )));
  }

  Ok(())
}
