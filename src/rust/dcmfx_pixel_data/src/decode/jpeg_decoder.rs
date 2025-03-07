#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use image::ImageBuffer;

use crate::{ColorImage, PixelDataDefinition, SingleChannelImage};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using jpeg-decoder.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (image_info, pixel_data) = decode(definition, data)?;

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if image_info.pixel_format == jpeg_decoder::PixelFormat::L8
    && pixel_data.len() == pixel_count
  {
    Ok(SingleChannelImage::Uint8(
      ImageBuffer::from_raw(width, height, pixel_data).unwrap(),
    ))
  } else if image_info.pixel_format == jpeg_decoder::PixelFormat::L16
    && pixel_data.len() == pixel_count * 2
  {
    let mut data = Vec::with_capacity(pixel_count);
    for chunk in pixel_data.chunks_exact(2) {
      data.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(SingleChannelImage::Uint16(
      ImageBuffer::from_raw(width, height, data).unwrap(),
    ))
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

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if image_info.pixel_format == jpeg_decoder::PixelFormat::RGB24
    && pixel_data.len() == pixel_count * 3
  {
    Ok(ColorImage::Uint8(
      ImageBuffer::from_raw(width, height, pixel_data).unwrap(),
    ))
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
    DataError::new_value_invalid(format!("Failed reading JPEG data: {}", e))
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
