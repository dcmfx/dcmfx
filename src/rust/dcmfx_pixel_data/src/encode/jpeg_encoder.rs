#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
  monochrome_image::MonochromeImageData,
};

use super::PixelDataEncodeConfig;

/// Returns the photometric interpretation used by an image encoded using
/// zune_jpeg.
///
pub fn encode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<PhotometricInterpretation, PixelDataEncodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => {
      Ok(photometric_interpretation.clone())
    }

    _ => Err(PixelDataEncodeError::NotSupported {
      details: format!(
        "Encoding photometric interpretation '{}' is not supported",
        photometric_interpretation
      ),
    }),
  }
}

/// Encodes a [`MonochromeImage`] into JPEG Baseline (Process 1) raw bytes.
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
      details: format!(
        "Photometric interpretation '{}' is not able to be encoded into JPEG \
         pixel data",
        image_pixel_module.photometric_interpretation()
      ),
    }),
  }
}

/// Encodes a [`ColorImage`] into JPEG Baseline (Process 1) raw bytes.
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
        color_space: ColorSpace::Ybr,
      },
      PhotometricInterpretation::YbrFull,
    ) => encode(data, width, height, jpeg_encoder::ColorType::Ycbcr, quality),

    _ => Err(PixelDataEncodeError::NotSupported {
      details: format!(
        "Photometric interpretation '{}' is not able to be encoded into JPEG \
         pixel data",
        image_pixel_module.photometric_interpretation()
      ),
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
