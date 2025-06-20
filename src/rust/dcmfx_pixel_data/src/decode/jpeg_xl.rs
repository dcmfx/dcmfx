#[cfg(not(feature = "std"))]
use alloc::format;

use crate::{
  PixelDataDecodeError, iods::image_pixel_module::PhotometricInterpretation,
};

/// Returns the photometric interpretation resulting from decoding JPEG XL.
///
pub fn decode_photometric_interpretation(
  photometric_interpretation: &PhotometricInterpretation,
) -> Result<&PhotometricInterpretation, PixelDataDecodeError> {
  match photometric_interpretation {
    PhotometricInterpretation::Monochrome1 { .. }
    | PhotometricInterpretation::Monochrome2 { .. }
    | PhotometricInterpretation::Rgb => Ok(photometric_interpretation),

    PhotometricInterpretation::YbrFull422
    | PhotometricInterpretation::YbrRct
    | PhotometricInterpretation::Xyb => Ok(&PhotometricInterpretation::Rgb),

    _ => Err(PixelDataDecodeError::ImagePixelModuleNotSupported {
      details: format!(
        "Photometric interpretation '{}' is not supported",
        photometric_interpretation
      ),
    }),
  }
}
