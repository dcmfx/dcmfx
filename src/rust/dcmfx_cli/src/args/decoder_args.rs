use clap::{Args, ValueEnum, builder::PossibleValue};

use dcmfx::pixel_data::{
  PixelDataDecodeConfig,
  decode::{HighThroughputJpeg2000Decoder, JpegXlDecoder},
};

#[derive(Args, Debug)]
pub struct DecoderArgs {
  #[arg(
    long,
    help_heading = "Pixel Data Decoding",
    help = "The library to use for decoding High-Throughput JPEG 2000 pixel \
      data. The OpenJPH library is preferred because it is the fastest \
      available decoder. However, WASM builds of DCMfx always use OpenJPEG and \
      so testing that library via the CLI tool is sometimes useful.\n\
      \n\
      There can be very slight differences in output between decoders when \
      decoding lossy High-Throughput JPEG 2000.",
    default_value_t = HighThroughputJpeg2000DecoderArg::OpenJph
  )]
  high_throughput_jpeg_2000_decoder: HighThroughputJpeg2000DecoderArg,

  #[arg(
    long,
    help_heading = "Pixel Data Decoding",
    help = "The library to use for decoding JPEG XL pixel data. The libjxl \
      library is preferred because it is the reference implementation and is \
      the fastest available decoder. However, WASM builds of DCMfx always use \
      jxl-oxide and so testing that library via the CLI tool is sometimes \
      useful.\n\
      \n\
      There should be no difference in output between decoders.",
    default_value_t = JpegXlDecoderArg::LibJxl
  )]
  jpeg_xl_decoder: JpegXlDecoderArg,
}

impl DecoderArgs {
  pub fn pixel_data_decode_config(&self) -> PixelDataDecodeConfig {
    PixelDataDecodeConfig {
      high_throughput_jpeg_2000_decoder: self
        .high_throughput_jpeg_2000_decoder
        .into(),
      jpeg_xl_decoder: self.jpeg_xl_decoder.into(),
    }
  }
}

/// Enum for specifying the decoder to use for High-Throughput JPEG 2000 pixel
/// data.
///
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HighThroughputJpeg2000DecoderArg {
  OpenJpeg,
  OpenJph,
}

impl From<HighThroughputJpeg2000DecoderArg> for HighThroughputJpeg2000Decoder {
  fn from(value: HighThroughputJpeg2000DecoderArg) -> Self {
    match value {
      HighThroughputJpeg2000DecoderArg::OpenJpeg => {
        HighThroughputJpeg2000Decoder::OpenJpeg
      }
      HighThroughputJpeg2000DecoderArg::OpenJph => {
        HighThroughputJpeg2000Decoder::OpenJph
      }
    }
  }
}

impl ValueEnum for HighThroughputJpeg2000DecoderArg {
  fn value_variants<'a>() -> &'a [Self] {
    &[Self::OpenJpeg, Self::OpenJph]
  }

  fn to_possible_value(&self) -> Option<PossibleValue> {
    Some(match self {
      Self::OpenJpeg => PossibleValue::new("openjpeg").help(
        "Use OpenJPEG for decoding High-Throughput JPEG 2000 pixel data.",
      ),
      Self::OpenJph => PossibleValue::new("openjph")
        .help("Use OpenJPH for decoding High-Throughput JPEG 2000 pixel data."),
    })
  }
}

impl core::fmt::Display for HighThroughputJpeg2000DecoderArg {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      HighThroughputJpeg2000DecoderArg::OpenJpeg => write!(f, "openjpeg"),
      HighThroughputJpeg2000DecoderArg::OpenJph => write!(f, "openjph"),
    }
  }
}

/// Enum for specifying the decoder to use for JPEG XL pixel data.
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
        .help("Use libjxl for decoding JPEG XL pixel data."),
      Self::JxlOxide => PossibleValue::new("jxl-oxide")
        .help("Use jxl-oxide for decoding JPEG XL pixel data."),
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
