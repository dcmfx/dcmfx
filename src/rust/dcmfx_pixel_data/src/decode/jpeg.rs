use image::DynamicImage;

use dcmfx_core::DataError;

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};

/// Decodes single channel JPEG pixel data.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  match decode(definition, data)? {
    DynamicImage::ImageLuma8(gray) => Ok(SingleChannelImage::Uint8(gray)),

    _ => Err(DataError::new_value_invalid(
      "JPEG pixel data is not single channel".to_string(),
    )),
  }
}

/// Decodes color JPEG pixel data.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  match decode(definition, data)? {
    DynamicImage::ImageRgb8(gray) => Ok(ColorImage::Uint8(gray)),

    _ => Err(DataError::new_value_invalid(
      "JPEG pixel data is not RGB".to_string(),
    )),
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<DynamicImage, DataError> {
  let img = image::load_from_memory_with_format(data, image::ImageFormat::Jpeg)
    .map_err(|_| {
      DataError::new_value_invalid("Invalid JPEG pixel data".to_string())
    })?;

  if img.width() != definition.columns.into()
    || img.height() != definition.rows.into()
  {
    return Err(DataError::new_value_invalid(
      "JPEG pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok(img)
}
