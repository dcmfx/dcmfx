#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use dcmfx_core::DataError;

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    ImagePixelModule, PhotometricInterpretation, SamplesPerPixel,
  },
};

/// Returns the photometric interpretation used by data decoded using zune-jpeg.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull
    | PhotometricInterpretation::YbrFull422 => Ok(photometric_interpretation),

    _ => Err(PixelDataDecodeError::NotSupported {
      details: format!(
        "Decoding photometric interpretation '{}' is not supported",
        photometric_interpretation
      ),
    }),
  }
}

/// Decodes monochrome JPEG pixel data using zune-jpeg.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, DataError> {
  if image_pixel_module.samples_per_pixel() != SamplesPerPixel::One {
    return Err(DataError::new_value_unsupported(
      "Samples per pixel must be one for monochrome JPEG decode".to_string(),
    ));
  }

  match image_pixel_module.photometric_interpretation() {
    PhotometricInterpretation::Monochrome1
    | PhotometricInterpretation::Monochrome2 => {
      let pixels = decode(
        image_pixel_module,
        data,
        zune_core::colorspace::ColorSpace::Luma,
      )?;

      MonochromeImage::new_u8(
        image_pixel_module.columns(),
        image_pixel_module.rows(),
        pixels,
        image_pixel_module.bits_stored(),
        image_pixel_module
          .photometric_interpretation()
          .is_monochrome1(),
      )
    }

    _ => Err(DataError::new_value_unsupported(format!(
      "Photometric interpretation '{}' is not supported for monochrome JPEG \
       decode",
      image_pixel_module.photometric_interpretation()
    ))),
  }
}

/// Decodes color JPEG pixel data using zune-jpeg.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, DataError> {
  if u8::from(image_pixel_module.samples_per_pixel()) != 3 {
    return Err(DataError::new_value_unsupported(
      "Samples per pixel must be three for color JPEG decode".to_string(),
    ));
  }

  let (zune_color_space, output_color_space) =
    match image_pixel_module.photometric_interpretation() {
      PhotometricInterpretation::Rgb => {
        (zune_core::colorspace::ColorSpace::RGB, ColorSpace::Rgb)
      }
      PhotometricInterpretation::YbrFull => (
        zune_core::colorspace::ColorSpace::YCbCr,
        ColorSpace::Ybr { is_422: false },
      ),
      PhotometricInterpretation::YbrFull422 => (
        zune_core::colorspace::ColorSpace::YCbCr,
        ColorSpace::Ybr { is_422: true },
      ),
      _ => {
        return Err(DataError::new_value_unsupported(format!(
          "Photometric interpretation '{}' is not supported for color JPEG \
           decode",
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
