use image::ImageBuffer;

use crate::{
  BitsAllocated, ColorImage, PixelDataDefinition, SingleChannelImage,
};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using CharLS.
///
pub fn decode_single_channel(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(data)?;

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if definition.bits_allocated == BitsAllocated::Eight
    && pixels.len() == pixel_count
  {
    Ok(SingleChannelImage::Uint8(
      ImageBuffer::from_raw(width, height, pixels).unwrap(),
    ))
  } else if definition.bits_allocated == BitsAllocated::Sixteen
    && pixels.len() == pixel_count * 2
  {
    let mut data = Vec::with_capacity(pixels.len() / 2);
    for chunk in pixels.chunks_exact(2) {
      data.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(SingleChannelImage::Uint16(
      ImageBuffer::from_raw(width, height, data).unwrap(),
    ))
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not single channel".to_string(),
    ))
  }
}

/// Decodes color pixel data using CharLS.
///
pub fn decode_color(
  definition: &PixelDataDefinition,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let pixels = decode(data)?;

  let pixel_count = definition.pixel_count();
  let width = definition.columns as u32;
  let height = definition.rows as u32;

  if definition.bits_allocated == BitsAllocated::Eight
    && pixels.len() == pixel_count * 3
  {
    Ok(ColorImage::Uint8(
      ImageBuffer::from_raw(width, height, pixels).unwrap(),
    ))
  } else if definition.bits_allocated == BitsAllocated::Sixteen
    && pixels.len() == pixel_count * 6
  {
    let mut data = Vec::with_capacity(pixels.len() / 2);
    for chunk in pixels.chunks_exact(2) {
      data.push(u16::from_le_bytes([chunk[0], chunk[1]]));
    }

    Ok(ColorImage::Uint16(
      ImageBuffer::from_raw(width, height, data).unwrap(),
    ))
  } else {
    Err(DataError::new_value_invalid(
      "JPEG LS pixel data is not color".to_string(),
    ))
  }
}

fn decode(data: &[u8]) -> Result<Vec<u8>, DataError> {
  let mut charls = charls::CharLS::default();

  let data = charls.decode(data).map_err(|e| {
    DataError::new_value_invalid(format!("Failed reading JPEG LS data: {}", e))
  })?;

  Ok(data)
}
