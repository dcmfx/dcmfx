#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec};

use jxl_oxide::{FrameBufferSample, JxlImage, Render, image::BitDepth};

use dcmfx_core::DataError;

use crate::{
  BitsAllocated, ColorImage, ColorSpace, PixelDataDefinition,
  SingleChannelImage,
};

/// Decodes single channel pixel data using jxl-oxide.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (jxl_image, jxl_render) = decode(definition, data)?;

  let width = definition.columns();
  let height = definition.rows();

  match (
    definition.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (BitsAllocated::Eight, BitDepth::IntegerSample { bits_per_sample: 8 }) => {
      let mut buffer = vec![0u8; definition.pixel_count()];
      render_samples(&jxl_render, &mut buffer)?;

      SingleChannelImage::new_u8(width, height, buffer)
    }

    (
      BitsAllocated::Sixteen,
      BitDepth::IntegerSample {
        bits_per_sample: 16,
      },
    ) => {
      let mut buffer = vec![0u16; definition.pixel_count()];
      render_samples(&jxl_render, &mut buffer)?;

      SingleChannelImage::new_u16(width, height, buffer)
    }

    _ => Err(DataError::new_value_invalid(
      "JPEG XL pixel data does not contain a supported single channel image"
        .to_string(),
    )),
  }
}

/// Decodes color pixel data using jxl-oxide.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  if definition.bits_allocated() == BitsAllocated::One {
    return Err(DataError::new_value_invalid(
      "JPEG XL does not support 1-bit pixel data".to_string(),
    ));
  }

  let (jxl_image, jxl_render) = decode(definition, data)?;
  let width = definition.columns();
  let height = definition.rows();

  match (
    definition.bits_allocated(),
    jxl_image.image_header().metadata.bit_depth,
  ) {
    (BitsAllocated::Eight, BitDepth::IntegerSample { bits_per_sample: 8 }) => {
      let mut buffer = vec![0u8; definition.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u8(width, height, buffer, ColorSpace::RGB)
    }

    (
      BitsAllocated::Sixteen,
      BitDepth::IntegerSample {
        bits_per_sample: 16,
      },
    ) => {
      let mut buffer = vec![0u16; definition.pixel_count() * 3];
      render_samples(&jxl_render, &mut buffer)?;

      ColorImage::new_u16(width, height, buffer, ColorSpace::RGB)
    }

    _ => Err(DataError::new_value_invalid(
      "JPEG XL pixel data does not contain a supported color image".to_string(),
    )),
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(JxlImage, Render), DataError> {
  if definition.pixel_representation().is_signed() {
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

  if image.width() != definition.columns().into()
    || image.height() != definition.rows().into()
  {
    return Err(DataError::new_value_invalid(
      "JPEG XL pixel data has incorrect dimensions".to_string(),
    ));
  }

  // Convert CMYK to sRGB
  if image.pixel_format().has_black() {
    image.request_color_encoding(jxl_oxide::EnumColourEncoding::srgb(
      jxl_oxide::RenderingIntent::Relative,
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
