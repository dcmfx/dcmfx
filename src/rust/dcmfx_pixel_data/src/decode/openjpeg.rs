use image::ImageBuffer;
use jpeg2k::*;

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using OpenJPEG.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(definition, data)?;

  match pixels.data {
    jpeg2k::ImagePixelData::L8(data) => Ok(SingleChannelImage::Uint8(
      ImageBuffer::from_raw(pixels.width, pixels.height, data).unwrap(),
    )),

    jpeg2k::ImagePixelData::L16(data) => Ok(SingleChannelImage::Uint16(
      ImageBuffer::from_raw(pixels.width, pixels.height, data).unwrap(),
    )),

    _ => Err(DataError::new_value_invalid(
      "JPEG 2000 pixel data is not single channel".to_string(),
    )),
  }
}

/// Decodes color pixel data using OpenJPEG.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(definition, data)?;

  match pixels.data {
    jpeg2k::ImagePixelData::Rgb8(data) => Ok(ColorImage::Uint8(
      ImageBuffer::from_raw(pixels.width, pixels.height, data).unwrap(),
    )),

    jpeg2k::ImagePixelData::Rgb16(data) => Ok(ColorImage::Uint16(
      ImageBuffer::from_raw(pixels.width, pixels.height, data).unwrap(),
    )),

    _ => Err(DataError::new_value_invalid(
      "JPEG 2000 pixel data is not RGB".to_string(),
    )),
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<jpeg2k::ImageData, DataError> {
  let img = Image::from_bytes(data).map_err(|_| {
    DataError::new_value_invalid("Invalid JPEG 2000 pixel data".to_string())
  })?;

  let pixels = img.get_pixels(None).map_err(|_| {
    DataError::new_value_invalid("Failed reading JPEG 2000 pixels".to_string())
  })?;

  if pixels.width != definition.columns.into()
    || pixels.height != definition.rows.into()
  {
    return Err(DataError::new_value_invalid(
      "JPEG 2000 pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok(pixels)
}
