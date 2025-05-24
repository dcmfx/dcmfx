use clap::{ValueEnum, builder::PossibleValue};

use dcmfx::pixel_data::iods::image_pixel_module::PhotometricInterpretation;

/// Enum for specifying a monochrome photometric interpretation as a CLI
/// argument.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhotometricInterpretationMonochromeArg {
  PassThrough,
  Monochrome1,
  Monochrome2,
}

impl ValueEnum for PhotometricInterpretationMonochromeArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[Self::PassThrough, Self::Monochrome1, Self::Monochrome2]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::PassThrough => PossibleValue::new("pass-through"),
      Self::Monochrome1 => PossibleValue::new("MONOCHROME1"),
      Self::Monochrome2 => PossibleValue::new("MONOCHROME2"),
    })
  }
}

impl PhotometricInterpretationMonochromeArg {
  /// Converts to the underlying [`PhotometricInterpretation`].
  ///
  pub fn as_photometric_interpretation(
    &self,
  ) -> Option<PhotometricInterpretation> {
    match self {
      PhotometricInterpretationMonochromeArg::PassThrough => None,
      PhotometricInterpretationMonochromeArg::Monochrome1 => {
        Some(PhotometricInterpretation::Monochrome1)
      }
      PhotometricInterpretationMonochromeArg::Monochrome2 => {
        Some(PhotometricInterpretation::Monochrome2)
      }
    }
  }
}

/// Enum for specifying a color photometric interpretation as a CLI argument.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PhotometricInterpretationColorArg {
  PassThrough,
  Rgb,
  YbrFull,
  YbrFull422,
  YbrIct,
  YbrRct,
}

impl ValueEnum for PhotometricInterpretationColorArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[
      Self::PassThrough,
      Self::Rgb,
      Self::YbrFull,
      Self::YbrFull422,
      Self::YbrIct,
      Self::YbrRct,
    ]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::PassThrough => PossibleValue::new("pass-through"),
      Self::Rgb => PossibleValue::new("RGB"),
      Self::YbrFull => PossibleValue::new("YBR_FULL"),
      Self::YbrFull422 => PossibleValue::new("YBR_FULL_422"),
      Self::YbrIct => PossibleValue::new("YBR_ICT"),
      Self::YbrRct => PossibleValue::new("YBR_RCT"),
    })
  }
}

impl PhotometricInterpretationColorArg {
  /// Converts to the underlying [`PhotometricInterpretation`].
  ///
  pub fn as_photometric_interpretation(
    &self,
  ) -> Option<PhotometricInterpretation> {
    match self {
      PhotometricInterpretationColorArg::PassThrough => None,
      PhotometricInterpretationColorArg::Rgb => {
        Some(PhotometricInterpretation::Rgb)
      }
      PhotometricInterpretationColorArg::YbrFull => {
        Some(PhotometricInterpretation::YbrFull)
      }
      PhotometricInterpretationColorArg::YbrFull422 => {
        Some(PhotometricInterpretation::YbrFull422)
      }
      PhotometricInterpretationColorArg::YbrIct => {
        Some(PhotometricInterpretation::YbrIct)
      }
      PhotometricInterpretationColorArg::YbrRct => {
        Some(PhotometricInterpretation::YbrRct)
      }
    }
  }
}
