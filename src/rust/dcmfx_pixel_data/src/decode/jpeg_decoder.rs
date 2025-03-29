#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use crate::{ColorImage, ColorSpace, PixelDataDefinition, SingleChannelImage};
use dcmfx_core::DataError;

use super::vec_cast;

/// Decodes single channel pixel data using jpeg-decoder.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (pixels, pixel_format) = decode(definition, data)?;

  let width = definition.columns();
  let height = definition.rows();

  if pixel_format == jpeg_decoder::PixelFormat::L8 {
    SingleChannelImage::new_u8(width, height, pixels)
  } else if pixel_format == jpeg_decoder::PixelFormat::L16 {
    let data = unsafe { vec_cast::<u8, u16>(pixels) };
    SingleChannelImage::new_u16(width, height, data)
  } else {
    Err(DataError::new_value_invalid(format!(
      "JPEG Lossless pixel format '{:?}' is not supported for single channel images",
      pixel_format
    )))
  }
}

/// Decodes color pixel data using jpeg-decoder.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let (pixels, pixel_format) = decode(definition, data)?;

  let width = definition.columns();
  let height = definition.rows();

  if pixel_format == jpeg_decoder::PixelFormat::RGB24 {
    ColorImage::new_u8(width, height, pixels, ColorSpace::RGB)
  } else {
    Err(DataError::new_value_invalid(format!(
      "JPEG Lossless pixel format '{:?}' is not supported for color images",
      pixel_format
    )))
  }
}

fn decode(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<(Vec<u8>, jpeg_decoder::PixelFormat), DataError> {
  let mut decoder = jpeg_decoder::Decoder::new(data);

  decoder.read_info().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG Lossless pixel data decoding failed with '{}'",
      e
    ))
  })?;

  let image_info = decoder.info().unwrap();

  if image_info.width != definition.columns()
    || image_info.height != definition.rows()
  {
    return Err(DataError::new_value_invalid(
      "JPEG Lossless pixel data has incorrect dimensions".to_string(),
    ));
  }

  let pixels = decoder.decode().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG Lossless pixel data decoding failed with '{}'",
      e
    ))
  })?;

  Ok((pixels, image_info.pixel_format))
}
