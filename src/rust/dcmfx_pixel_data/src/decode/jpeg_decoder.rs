#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use crate::{
  ColorImage, ColorSpace, SingleChannelImage, iods::ImagePixelModule,
};
use dcmfx_core::DataError;

/// Decodes single channel pixel data using jpeg-decoder.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let (pixels, pixel_format) = decode(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  if pixel_format == jpeg_decoder::PixelFormat::L8 {
    SingleChannelImage::new_u8(
      width,
      height,
      pixels,
      bits_stored,
      is_monochrome1,
    )
  } else if pixel_format == jpeg_decoder::PixelFormat::L16 {
    let data = bytemuck::cast_slice(&pixels).to_vec();
    SingleChannelImage::new_u16(
      width,
      height,
      data,
      bits_stored,
      is_monochrome1,
    )
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
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let (pixels, pixel_format) = decode(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();

  if pixel_format == jpeg_decoder::PixelFormat::RGB24 {
    ColorImage::new_u8(width, height, pixels, ColorSpace::RGB, bits_stored)
  } else {
    Err(DataError::new_value_invalid(format!(
      "JPEG Lossless pixel format '{:?}' is not supported for color images",
      pixel_format
    )))
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
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

  if image_info.width != image_pixel_module.columns()
    || image_info.height != image_pixel_module.rows()
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
