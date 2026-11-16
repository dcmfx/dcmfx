use clap::{Args, ValueEnum, builder::PossibleValue};

use dcmfx::pixel_data::{PixelDataEncodeConfig, encode::JpegBaselineEncoder};

#[derive(Args, Debug)]
pub struct EncoderArgs {
  #[arg(
    long,
    help_heading = "Pixel Data Encoding",
    help = "The library to use for encoding JPEG Baseline 8-bit pixel data. \
      This also applies when encoding into the JPEG XL JPEG Recompression \
      transfer syntax. The libjpeg-turbo library is preferred because it is \
      faster. However, WASM builds of DCMfx always use jpeg-encoder and so \
      testing that library via the CLI tool is sometimes useful.\n\
      \n\
      There can be slight differences in output between encoders.",
    default_value_t = JpegBaselineEncoderArg::LibJpegTurbo
  )]
  jpeg_baseline_encoder: JpegBaselineEncoderArg,
}

impl EncoderArgs {
  pub fn pixel_data_encode_config(&self) -> PixelDataEncodeConfig {
    let mut config = PixelDataEncodeConfig::default();

    config.set_jpeg_baseline_encoder(self.jpeg_baseline_encoder.into());

    config
  }
}

/// Enum for specifying the encoder to use for JPEG Baseline 8-bit pixel data.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JpegBaselineEncoderArg {
  JpegEncoder,
  LibJpegTurbo,
}

impl From<JpegBaselineEncoderArg> for JpegBaselineEncoder {
  fn from(value: JpegBaselineEncoderArg) -> Self {
    match value {
      JpegBaselineEncoderArg::JpegEncoder => JpegBaselineEncoder::JpegEncoder,
      JpegBaselineEncoderArg::LibJpegTurbo => JpegBaselineEncoder::LibJpegTurbo,
    }
  }
}

impl ValueEnum for JpegBaselineEncoderArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[Self::JpegEncoder, Self::LibJpegTurbo]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::JpegEncoder => PossibleValue::new("jpeg-encoder")
        .help("Use jpeg-encoder for encoding JPEG Baseline 8-bit pixel data."),
      Self::LibJpegTurbo => PossibleValue::new("libjpeg-turbo")
        .help("Use libjpeg-turbo for encoding JPEG Baseline 8-bit pixel data."),
    })
  }
}

impl core::fmt::Display for JpegBaselineEncoderArg {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      JpegBaselineEncoderArg::JpegEncoder => write!(f, "jpeg-encoder"),
      JpegBaselineEncoderArg::LibJpegTurbo => write!(f, "libjpeg-turbo"),
    }
  }
}
