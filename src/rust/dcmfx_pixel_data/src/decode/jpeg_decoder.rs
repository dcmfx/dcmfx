#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};
use dcmfx_core::DataError;

use super::vec_cast;

/// Decodes single channel pixel data using jpeg-decoder.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (image_info, pixel_data) = decode(definition, data)?;

  let width = definition.columns;
  let height = definition.rows;

  if image_info.pixel_format == jpeg_decoder::PixelFormat::L8 {
    SingleChannelImage::new_u8(width, height, pixel_data)
  } else if image_info.pixel_format == jpeg_decoder::PixelFormat::L16 {
    let data = unsafe { vec_cast::<u8, u16>(pixel_data) };
    SingleChannelImage::new_u16(width, height, data)
  } else {
    Err(DataError::new_value_invalid(
      "JPEG pixel data is not single channel".to_string(),
    ))
  }
}

/// Decodes color pixel data using jpeg-decoder.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let (image_info, pixel_data) = decode(definition, data)?;

  let width = definition.columns;
  let height = definition.rows;

  if image_info.pixel_format == jpeg_decoder::PixelFormat::RGB24 {
    ColorImage::new_u8(width, height, pixel_data)
  } else {
    Err(DataError::new_value_invalid(
      "JPEG pixel data is not color".to_string(),
    ))
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(jpeg_decoder::ImageInfo, Vec<u8>), DataError> {
  let mut decoder = jpeg_decoder::Decoder::new(data);

  let pixels = decoder.decode().map_err(|e| {
    DataError::new_value_invalid(format!(
      "Failed reading JPEG data with '{}'",
      e
    ))
  })?;

  let image_info = decoder.info().unwrap();

  if image_info.width != definition.columns
    || image_info.height != definition.rows
  {
    return Err(DataError::new_value_invalid(
      "JPEG pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok((image_info, pixels))
}
