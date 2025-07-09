use clap::{Args, ValueEnum, builder::PossibleValue};

use dcmfx::pixel_data::{PixelDataDecodeConfig, decode::JpegXlDecoder};

#[derive(Args, Debug)]
pub struct DecoderArgs {
  #[arg(
    long,
    help_heading = "Pixel Data Decoding",
    help = "The library to use for decoding JPEG XL pixel data. The libjxl \
      library is preferred because it is the reference implementation and is \
      the fastest available decoder. However, WASM builds of DCMfx always use \
      jxl-oxide and so testing that library via the CLI tool is sometimes \
      useful.\n\
      \n\
      There should not be any difference in output between decoders.",
    default_value_t = JpegXlDecoderArg::LibJxl
  )]
  jpeg_xl_decoder: JpegXlDecoderArg,
}

impl DecoderArgs {
  pub fn pixel_data_decode_config(&self) -> PixelDataDecodeConfig {
    PixelDataDecodeConfig {
      jpeg_xl_decoder: self.jpeg_xl_decoder.into(),
    }
  }
}

/// Enum for specifying the decoder to use for JPEG XL image data.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JpegXlDecoderArg {
  LibJxl,
  JxlOxide,
}

impl From<JpegXlDecoderArg> for JpegXlDecoder {
  fn from(value: JpegXlDecoderArg) -> Self {
    match value {
      JpegXlDecoderArg::LibJxl => JpegXlDecoder::LibJxl,
      JpegXlDecoderArg::JxlOxide => JpegXlDecoder::JxlOxide,
    }
  }
}

impl ValueEnum for JpegXlDecoderArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[Self::LibJxl, Self::JxlOxide]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::LibJxl => PossibleValue::new("libjxl")
        .help("Use libjxl for decoding JPEG XL image data."),
      Self::JxlOxide => PossibleValue::new("jxl-oxide")
        .help("Use jxl-oxide for decoding JPEG XL image data."),
    })
  }
}

impl core::fmt::Display for JpegXlDecoderArg {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      JpegXlDecoderArg::LibJxl => write!(f, "libjxl"),
      JpegXlDecoderArg::JxlOxide => write!(f, "jxl-oxide"),
    }
  }
}
