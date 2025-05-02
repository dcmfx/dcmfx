#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, SingleChannelImage,
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
};

/// Decodes single channel JPEG pixel data using zune-jpeg.
///
pub fn decode_single_channel(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<SingleChannelImage, DataError> {
  let pixels = decode(
    image_pixel_module,
    data,
    zune_core::colorspace::ColorSpace::Luma,
  )?;

  SingleChannelImage::new_u8(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    image_pixel_module.bits_stored(),
    image_pixel_module
      .photometric_interpretation()
      .is_monochrome1(),
  )
}

/// Decodes color JPEG pixel data using zune-jpeg.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  let (zune_color_space, output_color_space) =
    match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Rgb => {
        (zune_core::colorspace::ColorSpace::RGB, ColorSpace::RGB)
      }
      PhotometricInterpretation::YbrFull => {
        (zune_core::colorspace::ColorSpace::YCbCr, ColorSpace::YBR)
      }
      PhotometricInterpretation::YbrFull422 => {
        (zune_core::colorspace::ColorSpace::YCbCr, ColorSpace::YBR422)
      }
      _ => {
        return Err(DataError::new_value_unsupported(format!(
          "Photometric interpretation '{}' is not supported for JPEG decode",
          image_pixel_module.photometric_interpretation()
        )));
      }
    };

  let pixels = decode(image_pixel_module, data, zune_color_space)?;

  ColorImage::new_u8(
    image_pixel_module.columns(),
    image_pixel_module.rows(),
    pixels,
    output_color_space,
    image_pixel_module.bits_stored(),
  )
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
  color_space: zune_core::colorspace::ColorSpace,
) -> Result<Vec<u8>, DataError> {
  let mut decoder = zune_jpeg::JpegDecoder::new(data);

  decoder
    .set_options(decoder.get_options().jpeg_set_out_colorspace(color_space));

  decoder.decode_headers().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG pixel data decoding failed with '{}'",
      e
    ))
  })?;

  if decoder.info().unwrap().width != image_pixel_module.columns()
    || decoder.info().unwrap().height != image_pixel_module.rows()
  {
    return Err(DataError::new_value_invalid(
      "JPEG pixel data has incorrect dimensions".to_string(),
    ));
  }

  decoder.decode().map_err(|e| {
    DataError::new_value_invalid(format!(
      "JPEG pixel data decoding failed with '{}'",
      e
    ))
  })
}
