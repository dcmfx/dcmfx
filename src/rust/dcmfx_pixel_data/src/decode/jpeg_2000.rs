#[cfg(not(feature = "std"))]
use alloc::format;

use crate::{
  PixelDataDecodeError, iods::image_pixel_module::PhotometricInterpretation,
};

/// Returns the photometric interpretation used by decoded JPEG 2000 pixel data.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::PaletteColor { .. }
    | PhotometricInterpretation::Rgb
    | PhotometricInterpretation::YbrFull => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrIct | PhotometricInterpretation::YbrRct => {
      Ok(&PhotometricInterpretation::Rgb)
    }

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{photometric_interpretation}' is not \
         supported"
      ),
    }),
  }
}
