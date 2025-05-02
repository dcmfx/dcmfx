#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{ImagePixelModule, PhotometricInterpretation},
};

/// Returns the photometric interpretation used by data decoded using
/// jpeg-decoder.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull => Ok(&PhotometricInterpretation::Rgb),

    _ => {
      Err(PixelDataDecodeError::NotSupported {
        details: format!(
          "Decoding photometric interpretation '{}' is not supported",
          photometric_interpretation
        ),
      })
    }
  }
}

/// Decodes monochrome pixel data using jpeg-decoder.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, DataError> {
  let (pixels, pixel_format) = decode(image_pixel_module, data)?;

  let width = image_pixel_module.columns();
  let height = image_pixel_module.rows();
  let bits_stored = image_pixel_module.bits_stored();
  let is_monochrome1 = image_pixel_module
    .photometric_interpretation()
    .is_monochrome1();

  match (
    image_pixel_module.photometric_interpretation(),
    pixel_format,
  ) {
    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      jpeg_decoder::PixelFormat::L8,
    ) => MonochromeImage::new_u8(
      width,
      height,
      pixels,
      bits_stored,
      is_monochrome1,
    ),

    (
      PhotometricInterpretation::Monochrome1
      | PhotometricInterpretation::Monochrome2,
      jpeg_decoder::PixelFormat::L16,
    ) => {
      let data = bytemuck::cast_slice(&pixels).to_vec();
      MonochromeImage::new_u16(width, height, data, bits_stored, is_monochrome1)
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for JPEG Lossless decode \
       when decoded pixel format is '{:?}'",
      image_pixel_module.photometric_interpretation(),
      pixel_format
    ))),
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

  match (
    image_pixel_module.photometric_interpretation(),
    pixel_format,
  ) {
    (
      PhotometricInterpretation::PaletteColor { palette },
      jpeg_decoder::PixelFormat::L8,
    ) => ColorImage::new_palette8(
      width,
      height,
      pixels,
      palette.clone(),
      bits_stored,
    ),

    (
      PhotometricInterpretation::PaletteColor { palette },
      jpeg_decoder::PixelFormat::L16,
    ) => {
      let data = bytemuck::cast_slice(&pixels).to_vec();
      ColorImage::new_palette16(
        width,
        height,
        data,
        palette.clone(),
        bits_stored,
      )
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      jpeg_decoder::PixelFormat::RGB24,
    ) => {
      ColorImage::new_u8(width, height, pixels, ColorSpace::Rgb, bits_stored)
    }

    _ => Err(DataError::new_value_invalid(format!(
      "Photometric interpretation '{}' is invalid for JPEG Lossless decode \
       when decoded pixel format is '{:?}'",
      image_pixel_module.photometric_interpretation(),
      pixel_format
    ))),
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<(Vec<u8>, jpeg_decoder::PixelFormat), DataError> {
  let mut decoder = jpeg_decoder::Decoder::new(data);

  if image_pixel_module.is_color() {
    decoder.set_color_transform(jpeg_decoder::ColorTransform::RGB);
  }

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
