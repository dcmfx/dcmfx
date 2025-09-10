#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataDecodeError,
  iods::image_pixel_module::{
    BitsAllocated, ImagePixelModule, PhotometricInterpretation,
    PixelRepresentation,
  },
};

/// Returns the photometric interpretation used by data decoded using zune-jpeg.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull422 => Ok(photometric_interpretation),

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{photometric_interpretation}' is not \
         supported"
      ),
    }),
  }
}

/// Decodes monochrome JPEG pixel data using zune-jpeg.
///
pub fn decode_monochrome(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<MonochromeImage, PixelDataDecodeError> {
  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Monochrome1 {
        pixel_representation: PixelRepresentation::Unsigned,
      }
      | PhotometricInterpretation::Monochrome2 {
        pixel_representation: PixelRepresentation::Unsigned,
      },
      BitsAllocated::Eight,
    ) => {
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
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG monochrome decode not supported for photometric interpretation \
           '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

/// Decodes color JPEG pixel data using zune-jpeg.
///
pub fn decode_color(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
) -> Result<ColorImage, PixelDataDecodeError> {
  match (
    image_pixel_module.photometric_interpretation(),
    image_pixel_module.bits_allocated(),
  ) {
    (
      PhotometricInterpretation::Rgb | PhotometricInterpretation::YbrFull422,
      BitsAllocated::Eight,
    ) => {
      let (zune_color_space, output_color_space) =
        match image_pixel_module.photometric_interpretation() {
          PhotometricInterpretation::Rgb => {
            (zune_core::colorspace::ColorSpace::RGB, ColorSpace::Rgb)
          }
          PhotometricInterpretation::YbrFull422 => (
            zune_core::colorspace::ColorSpace::YCbCr,
            ColorSpace::Ybr { is_422: true },
          ),
          _ => unreachable!(),
        };

      let pixels = decode(image_pixel_module, data, zune_color_space)?;

      ColorImage::new_u8(
        image_pixel_module.columns(),
        image_pixel_module.rows(),
        pixels,
        output_color_space,
        image_pixel_module.bits_stored(),
      )
      .map_err(PixelDataDecodeError::ImageCreationFailed)
    }

    (photometric_interpretation, bits_allocated) => {
      Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
        details: format!(
          "JPEG color decode not supported for photometric interpretation \
           '{}', bits allocated '{}'",
          photometric_interpretation,
          u8::from(bits_allocated)
        ),
      })
    }
  }
}

fn decode(
  image_pixel_module: &ImagePixelModule,
  data: &[u8],
  color_space: zune_core::colorspace::ColorSpace,
) -> Result<Vec<u8>, PixelDataDecodeError> {
  let mut decoder = zune_jpeg::JpegDecoder::new(data);

  decoder
    .set_options(decoder.get_options().jpeg_set_out_colorspace(color_space));

  decoder
    .decode_headers()
    .map_err(|e| PixelDataDecodeError::DataInvalid {
      details: format!("JPEG header decode failed with '{e}'"),
    })?;

  if decoder.info().unwrap().width != image_pixel_module.columns()
    || decoder.info().unwrap().height != image_pixel_module.rows()
  {
    return Err(PixelDataDecodeError::DataInvalid {
      details: "JPEG image has incorrect dimensions".to_string(),
    });
  }

  decoder
    .decode()
    .map_err(|e| PixelDataDecodeError::DataInvalid {
      details: format!("JPEG decode failed with '{e}'"),
    })
}
