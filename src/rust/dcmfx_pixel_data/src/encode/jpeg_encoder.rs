#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataEncodeError,
  color_image::ColorImageData,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, PlanarConfiguration,
  },
  monochrome_image::MonochromeImageData,
};

use super::PixelDataEncodeConfig;

/// Returns the Image Pixel Module resulting from encoding using jpeg-encoder.
///
pub fn encode_image_pixel_module(
  mut image_pixel_module: ImagePixelModule,
) -> Result<ImagePixelModule, ()> {
  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrFull422 => (),

    _ => return Err(()),
  }

  image_pixel_module.set_planar_configuration(PlanarConfiguration::Interleaved);

  Ok(image_pixel_module)
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
    ) => encode(
      data,
      width,
      height,
      jpeg_encoder::ColorType::Luma,
      None,
      quality,
    ),

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
    ) => encode(
      data,
      width,
      height,
      jpeg_encoder::ColorType::Rgb,
      Some(jpeg_encoder::SamplingFactor::R_4_4_4),
      quality,
    ),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { .. },
      },
      PhotometricInterpretation::YbrFull,
    ) => encode(
      data,
      width,
      height,
      jpeg_encoder::ColorType::Ycbcr,
      Some(jpeg_encoder::SamplingFactor::R_4_4_4),
      quality,
    ),

    (
      ColorImageData::U8 {
        data,
        color_space: ColorSpace::Ybr { .. },
      },
      PhotometricInterpretation::YbrFull422,
    ) => encode(
      data,
      width,
      height,
      jpeg_encoder::ColorType::Ycbcr,
      Some(jpeg_encoder::SamplingFactor::R_4_2_2),
      quality,
    ),

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
  sampling_factor: Option<jpeg_encoder::SamplingFactor>,
  quality: u8,
) -> Result<Vec<u8>, PixelDataEncodeError> {
  let mut buffer = vec![];

  let mut encoder = jpeg_encoder::Encoder::new(&mut buffer, quality);

  if let Some(sampling_factor) = sampling_factor {
    encoder.set_sampling_factor(sampling_factor);
  }

  encoder
    .encode(data, width, height, color_type)
    .map_err(|e| PixelDataEncodeError::OtherError {
      name: "JPEG encode failed".to_string(),
      details: e.to_string(),
    })?;

  Ok(buffer)
}
