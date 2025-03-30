#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{ColorImage, ColorSpace, PixelDataDefinition, SingleChannelImage};

/// Decodes single channel JPEG pixel data using zune-jpeg.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels =
    decode(definition, data, zune_core::colorspace::ColorSpace::Luma)?;

  SingleChannelImage::new_u8(definition.columns(), definition.rows(), pixels)
}

/// Decodes color JPEG pixel data using zune-jpeg.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels =
    decode(definition, data, zune_core::colorspace::ColorSpace::RGB)?;

  ColorImage::new_u8(
    definition.columns(),
    definition.rows(),
    pixels,
    ColorSpace::RGB,
  )
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
  color_space: zune_core::colorspace::ColorSpace,
) -> Result<Vec<u8>, DataError> {
  let mut decoder = zune_jpeg::JpegDecoder::new(data);

  decoder
    .set_options(decoder.get_options().jpeg_set_out_colorspace(color_space));

  decoder.decode_headers().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG pixel data decoding failed with '{}'",
      e
    ))
  })?;

  if decoder.info().unwrap().width != definition.columns()
    || decoder.info().unwrap().height != definition.rows()
  {
    return Err(DataError::new_value_invalid(
      "JPEG pixel data has incorrect dimensions".to_string(),
    ));
  }

  decoder.decode().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG pixel data decoding failed with '{}'",
      e
    ))
  })
}
