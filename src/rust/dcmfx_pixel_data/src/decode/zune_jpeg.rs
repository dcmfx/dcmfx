#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};

/// Decodes single channel JPEG pixel data using zune-jpeg.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(definition, data)?;
  SingleChannelImage::new_u8(definition.columns, definition.rows, pixels)
}

/// Decodes color JPEG pixel data using zune-jpeg.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(definition, data)?;
  ColorImage::new_u8(definition.columns, definition.rows, pixels)
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<Vec<u8>, DataError> {
  let mut decoder = zune_jpeg::JpegDecoder::new(data);

  let img = decoder.decode().map_err(|_| {
    DataError::new_value_invalid("JPEG pixel data decoding failed".to_string())
  })?;

  if decoder.info().unwrap().width != definition.columns
    || decoder.info().unwrap().height != definition.rows
  {
    return Err(DataError::new_value_invalid(
      "JPEG pixel data has incorrect dimensions".to_string(),
    ));
  }

  Ok(img)
}
