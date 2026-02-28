#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, PixelRepresentation,
  },
};

/// Returns the photometric interpretation used by data decoded using
/// jpeg-decoder.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull => Ok(&PhotometricInterpretation::Rgb),

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{photometric_interpretation}' is not \
         supported"
      ),
    }),
  }
}

/// Decodes monochrome pixel data using jpeg-decoder.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, PixelDataDecodeError> {
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
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      jpeg_decoder::PixelFormat::L8,
    ) => MonochromeImage::new_u8(
      width,
      height,
      pixels,
      bits_stored,
      is_monochrome1,
    )
    .map_err(PixelDataDecodeError::ImageCreationFailed),

    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      jpeg_decoder::PixelFormat::L8,
    ) => {
      let data = bytemuck::cast_slice(&pixels).to_vec();
      MonochromeImage::new_i8(width, height, data, bits_stored, is_monochrome1)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      jpeg_decoder::PixelFormat::L16,
    ) => {
      let data = bytemuck::cast_slice(&pixels).to_vec();
      MonochromeImage::new_u16(width, height, data, bits_stored, is_monochrome1)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Signed,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Signed,
      },
      jpeg_decoder::PixelFormat::L16,
    ) => {
      let data = bytemuck::cast_slice(&pixels).to_vec();
      MonochromeImage::new_i16(width, height, data, bits_stored, is_monochrome1)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "JPEG Lossless monochrome decode not supported for photometric \
         interpretation '{}', decoded pixel format '{:?}'",
        image_pixel_module.photometric_interpretation(),
        pixel_format
      ),
    }),
  }
}

/// Decodes color pixel data using jpeg-decoder.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
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
    )
    .map_err(PixelDataDecodeError::ImageCreationFailed),

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
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull,
      jpeg_decoder::PixelFormat::RGB24,
    ) => {
      ColorImage::new_u8(width, height, pixels, ColorSpace::Rgb, bits_stored)
        .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "JPEG Lossless color decode not supported for photometric \
         interpretation '{}', decoded pixel format '{:?}'",
        image_pixel_module.photometric_interpretation(),
        pixel_format
      ),
    }),
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<(Vec<u8>, jpeg_decoder::PixelFormat), PixelDataDecodeError> {
  let mut decoder = jpeg_decoder::Decoder::new(data);

  if image_pixel_module.is_color() {
    decoder.set_color_transform(jpeg_decoder::ColorTransform::RGB);
  }

  decoder
    .read_info()
    .map_err(|e| PixelDataDecodeError::DataInvalid {
      details: format!("JPEG Lossless pixel data decode failed with '{e}'"),
    })?;

  let image_info = decoder.info().unwrap();

  if image_info.width != image_pixel_module.columns()
    || image_info.height != image_pixel_module.rows()
  {
    return Err(PixelDataDecodeError::DataInvalid {
      details: "JPEG Lossless pixel data has incorrect dimensions".to_string(),
    });
  }

  let pixels =
    decoder
      .decode()
      .map_err(|e| PixelDataDecodeError::DataInvalid {
        details: format!("JPEG Lossless pixel data decode failed with '{e}'"),
      })?;

  Ok((pixels, image_info.pixel_format))
}
