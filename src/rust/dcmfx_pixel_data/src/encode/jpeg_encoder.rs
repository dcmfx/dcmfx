#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
  monochrome_image::MonochromeImageData,
};

use super::PixelDataEncodeConfig;

/// Returns the photometric interpretation used by an image encoded using
/// jpeg-encoder.
///
pub fn encode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, ()> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull422 => {
      Ok(&PhotometricInterpretation::YbrFull)
    }

    PhotometricInterpretation::PaletteColor { .. } => {
      Ok(&PhotometricInterpretation::Rgb)
    }

    _ => Err(()),
  }
}

/// Encodes a [`MonochromeImage`] into JPEG Baseline (Process 1) raw bytes using
/// `jpeg-encoder`.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();
  let quality = encode_config.quality;

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      MonochromeImageData::U8(data),
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
    ) => encode(data, width, height, jpeg_encoder::ColorType::Luma, quality),

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: None,
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG Baseline (Process 1) raw bytes using
/// `jpeg-encoder`.
///
pub fn encode_color(
  image: &ColorImage,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let width = image.width();
  let height = image.height();
  let quality = encode_config.quality;

  match (
    image.data(),
    image_pixel_module.photometric_interpretation(),
  ) {
    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Rgb,
      },
      PhotometricInterpretation::Rgb,
    ) => encode(data, width, height, jpeg_encoder::ColorType::Rgb, quality),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr | ColorSpace::Ybr422,
      },
      PhotometricInterpretation::YbrFull,
    ) => encode(data, width, height, jpeg_encoder::ColorType::Ycbcr, quality),

    (
      ColorImageData::PaletteU8 { data, palette },
      PhotometricInterpretation::Rgb,
    ) if palette.int_max() <= 255 => {
      let mut rgb_data = Vec::with_capacity(data.len() * 3);

      for index in data {
        let pixel = palette.lookup(i64::from(*index));
        rgb_data.push(pixel[0] as u8);
        rgb_data.push(pixel[1] as u8);
        rgb_data.push(pixel[2] as u8);
      }

      encode(
        &rgb_data,
        width,
        height,
        jpeg_encoder::ColorType::Rgb,
        quality,
      )
    }

    _ => Err(PixelDataEncodeError::NotSupported {
      image_pixel_module: Box::new(image_pixel_module.clone()),
      input_bits_allocated: image.bits_allocated(),
      input_color_space: Some(image.color_space()),
    }),
  }
}

fn encode(
  data: &[u8],
  width: u16,
  height: u16,
  color_type: jpeg_encoder::ColorType,
  quality: u8,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut buffer = vec![];

  jpeg_encoder::Encoder::new(&mut buffer, quality)
    .encode(data, width, height, color_type)
    .map_err(|e| PixelDataEncodeError::OtherError {
      name: "JPEG encode failed".to_string(),
      details: e.to_string(),
    })?;

  Ok(buffer)
}
