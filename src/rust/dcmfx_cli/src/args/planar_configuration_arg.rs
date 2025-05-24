use clap::ValueEnum;

use dcmfx::pixel_data::iods::image_pixel_module::PlanarConfiguration;

/// Enum for specifying a planar configuration as a CLI argument.
///
#[derive(Debug, Clone, Copy, PartialEq, ValueEnum)]
pub enum PlanarConfigurationArg {
  /// The sample values for the first pixel are followed by the sample values
  /// for the second pixel, etc. For RGB images, this means the order of the
  /// pixel values encoded shall be R1, G1, B1, R2, G2, B2, …, etc.
  Interleaved,

  /// Each color plane shall be encoded contiguously. For RGB images, this means
  /// the order of the pixel values encoded is R1, R2, R3, …, G1, G2, G3, …, B1,
  /// B2, B3, etc.
  Separate,
}

impl From<PlanarConfigurationArg> for PlanarConfiguration {
  fn from(value: PlanarConfigurationArg) -> Self {
    match value {
      PlanarConfigurationArg::Interleaved => PlanarConfiguration::Interleaved,
      PlanarConfigurationArg::Separate => PlanarConfiguration::Separate,
    }
  }
}
