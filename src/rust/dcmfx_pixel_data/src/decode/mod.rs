#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{DcmfxError, TransferSyntax, transfer_syntax};

use crate::{
  ColorImage, MonochromeImage, PixelDataFrame,
  iods::{ImagePixelModule, image_pixel_module::PhotometricInterpretation},
};

#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod charls;
#[cfg(feature = "native")]
mod jpeg_2000;
mod jpeg_decoder;
mod jpeg_xl;
mod jxl_oxide;
#[cfg(feature = "native")]
mod libjpeg_12bit;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod libjxl;
mod native;
#[cfg(feature = "native")]
mod openjpeg;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod openjph;
mod rle_lossless;
mod zune_jpeg;

/// Configuration used when decoding pixel data.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelDataDecodeConfig {
  /// The library to use for decoding High-Throughput JPEG 2000 pixel data.
  /// Defaults to [`HighThroughputJpeg2000Decoder::OpenJph`] except on WASM
  /// where it defaults to [`HighThroughputJpeg2000Decoder::OpenJpeg`].
  ///
  pub high_throughput_jpeg_2000_decoder: HighThroughputJpeg2000Decoder,

  /// The library to use for decoding JPEG XL pixel data. Defaults to
  /// [`JpegXlDecoder::LibJxl`] except on WASM where it defaults to
  /// [`JpegXlDecoder::JxlOxide`].
  ///
  pub jpeg_xl_decoder: JpegXlDecoder,
}

impl Default for PixelDataDecodeConfig {
  fn default() -> Self {
    #[cfg(not(target_arch = "wasm32"))]
    let jpeg_xl_decoder = JpegXlDecoder::LibJxl;

    #[cfg(target_arch = "wasm32")]
    let jpeg_xl_decoder = JpegXlDecoder::JxlOxide;

    #[cfg(not(target_arch = "wasm32"))]
    let high_throughput_jpeg_2000_decoder =
      HighThroughputJpeg2000Decoder::OpenJph;

    #[cfg(target_arch = "wasm32")]
    let high_throughput_jpeg_2000_decoder =
      HighThroughputJpeg2000Decoder::OpenJpeg;

    Self {
      jpeg_xl_decoder,
      high_throughput_jpeg_2000_decoder,
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HighThroughputJpeg2000Decoder {
  OpenJpeg,
  OpenJph,
}

impl core::fmt::Display for HighThroughputJpeg2000Decoder {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::OpenJpeg => f.write_str("openjpeg"),
      Self::OpenJph => f.write_str("openjph"),
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum JpegXlDecoder {
  LibJxl,
  JxlOxide,
}

impl core::fmt::Display for JpegXlDecoder {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::LibJxl => f.write_str("libjxl"),
      Self::JxlOxide => f.write_str("jxl-oxide"),
    }
  }
}

/// Errors that can occur when decoding frames of image data in a specific
/// transfer syntax.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PixelDataDecodeError {
  /// The transfer syntax is not supported for decoding.
  TransferSyntaxNotSupported {
    transfer_syntax: &'static TransferSyntax,
  },

  /// The configuration of the Image Pixel Module is not supported for decoding,
  /// so decoding can't be attempted.
  ///
  /// For example, if the Image Pixel Module states that Bits Allocated is 32
  /// but the decoder only supports decoding up to 16 bits.
  ImagePixelModuleNotSupported { details: String },

  /// There was an error reading or parsing the provided raw pixel data, i.e. it
  /// is invalid for the given Image Pixel Module and transfer syntax.
  DataInvalid { details: String },

  /// Decode succeeded but there was an error when constructing the
  /// [`MonochromeImage`] or [`ColorImage`] to be returned.
  ImageCreationFailed(&'static str),

  /// The decoder requested in the decoding config is not available because it
  /// wasn't part of the build.
  DecoderNotAvailable { name: String },
}

impl PixelDataDecodeError {
  /// Returns the name of the pixel data encode error as a human-readable
  /// string.
  ///
  pub fn name(&self) -> String {
    match self {
      Self::TransferSyntaxNotSupported { .. } => {
        "Transfer syntax not supported".to_string()
      }
      Self::ImagePixelModuleNotSupported { .. } => {
        "Image pixel module not supported for decode".to_string()
      }
      Self::DataInvalid { .. } => "Data invalid".to_string(),
      Self::ImageCreationFailed { .. } => "Image creation failed".to_string(),
      Self::DecoderNotAvailable { .. } => "Decoder not available".to_string(),
    }
  }
}

impl core::fmt::Display for PixelDataDecodeError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::TransferSyntaxNotSupported { transfer_syntax } => {
        write!(
          f,
          "Transfer syntax '{}' is not supported",
          transfer_syntax.name
        )
      }
      Self::ImagePixelModuleNotSupported { details } => {
        write!(f, "Image pixel module not supported, details: {details}")
      }
      Self::DataInvalid { details } => {
        write!(f, "Data invalid, details: '{details}'")
      }
      Self::ImageCreationFailed(details) => {
        write!(f, "Image creation failed, details: '{details}'")
      }
      Self::DecoderNotAvailable { name } => {
        write!(f, "Decoder '{name}' not available")
      }
    }
  }
}

impl DcmfxError for PixelDataDecodeError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("Pixel data decode error {task_description}"),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match self {
      Self::TransferSyntaxNotSupported { transfer_syntax } => {
        lines.push(format!("  Transfer syntax: {}", transfer_syntax.name));
      }
      Self::ImagePixelModuleNotSupported { details }
      | Self::DataInvalid { details } => {
        lines.push(format!("  Details: {details}"));
      }
      Self::ImageCreationFailed(details) => {
        lines.push(format!("  Details: {details}"));
      }
      Self::DecoderNotAvailable { name } => {
        lines.push(format!("  Name: {name}"));
      }
    }

    lines
  }
}

/// Given an input photometric interpretation and transfer syntax, returns the
/// photometric interpretation of the decoded image data. This is limited to the
/// equivalent photometric interpretations that can be represented in a
/// [`MonochromeImage`] or [`ColorImage`].
///
#[allow(clippy::result_unit_err)]
pub fn decode_photometric_interpretation<'a>(
  photometric_interpretation: &'a PhotometricInterpretation,
  transfer_syntax: &'static TransferSyntax,
) -> Result<&'a PhotometricInterpretation, PixelDataDecodeError> {
  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => {
      native::decode_photometric_interpretation(photometric_interpretation)
    }

    &RLE_LOSSLESS => rle_lossless::decode_photometric_interpretation(
      photometric_interpretation,
    ),

    &JPEG_BASELINE_8BIT => {
      zune_jpeg::decode_photometric_interpretation(photometric_interpretation)
    }

    #[cfg(feature = "native")]
    &JPEG_EXTENDED_12BIT => libjpeg_12bit::decode_photometric_interpretation(
      photometric_interpretation,
    ),

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_photometric_interpretation(
        photometric_interpretation,
      )
    }

    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_photometric_interpretation(photometric_interpretation)
    }

    #[cfg(feature = "native")]
    &JPEG_2000
    | &JPEG_2000_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000
    | &HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
      jpeg_2000::decode_photometric_interpretation(photometric_interpretation)
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      jpeg_xl::decode_photometric_interpretation(photometric_interpretation)
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => {
      native::decode_photometric_interpretation(photometric_interpretation)
    }

    _ => {
      Err(PixelDataDecodeError::TransferSyntaxNotSupported { transfer_syntax })
    }
  }
}

/// Decodes a frame of monochrome pixel data into a [`MonochromeImage`]. The
/// returned image needs to have a grayscale pipeline applied in order to reach
/// final grayscale display values.
///
pub fn decode_monochrome(
  frame: &mut PixelDataFrame,
  transfer_syntax: &'static TransferSyntax,
  image_pixel_module: &ImagePixelModule,
  decode_config: &PixelDataDecodeConfig,
) -> Result<MonochromeImage, PixelDataDecodeError> {
  let frame_bit_offset = frame.bit_offset();
  let data = frame.combine_chunks();

  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => {
      native::decode_monochrome(image_pixel_module, data, frame_bit_offset)
    }

    &RLE_LOSSLESS => rle_lossless::decode_monochrome(image_pixel_module, data),

    &JPEG_BASELINE_8BIT => {
      zune_jpeg::decode_monochrome(image_pixel_module, data)
    }

    #[cfg(feature = "native")]
    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::decode_monochrome(image_pixel_module, data)
    }

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_monochrome(image_pixel_module, data)
    }

    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_monochrome(image_pixel_module, data)
    }

    #[cfg(feature = "native")]
    &JPEG_2000 | &JPEG_2000_LOSSLESS_ONLY => {
      openjpeg::decode_monochrome(image_pixel_module, data)
    }

    #[cfg(feature = "native")]
    &HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000 => {
      #[cfg(not(target_arch = "wasm32"))]
      if decode_config.high_throughput_jpeg_2000_decoder
        == HighThroughputJpeg2000Decoder::OpenJph
      {
        return openjph::decode_monochrome(image_pixel_module, data);
      }

      if decode_config.high_throughput_jpeg_2000_decoder
        == HighThroughputJpeg2000Decoder::OpenJpeg
      {
        return openjpeg::decode_monochrome(image_pixel_module, data);
      }

      Err(PixelDataDecodeError::DecoderNotAvailable {
        name: decode_config.high_throughput_jpeg_2000_decoder.to_string(),
      })
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
      if decode_config.jpeg_xl_decoder == JpegXlDecoder::LibJxl {
        return libjxl::decode_monochrome(image_pixel_module, data);
      }

      if decode_config.jpeg_xl_decoder == JpegXlDecoder::JxlOxide {
        return jxl_oxide::decode_monochrome(image_pixel_module, data);
      }

      Err(PixelDataDecodeError::DecoderNotAvailable {
        name: decode_config.jpeg_xl_decoder.to_string(),
      })
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => native::decode_monochrome(
      image_pixel_module,
      &inflate_frame_data(data, image_pixel_module)?,
      0,
    ),

    _ => {
      Err(PixelDataDecodeError::TransferSyntaxNotSupported { transfer_syntax })
    }
  }
}

/// Decodes a frame of color pixel data into a [`ColorImage`].
///
pub fn decode_color(
  frame: &mut PixelDataFrame,
  transfer_syntax: &'static TransferSyntax,
  image_pixel_module: &ImagePixelModule,
  decode_config: &PixelDataDecodeConfig,
) -> Result<ColorImage, PixelDataDecodeError> {
  let data = frame.combine_chunks();

  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => native::decode_color(image_pixel_module, data),

    &RLE_LOSSLESS => rle_lossless::decode_color(image_pixel_module, data),

    &JPEG_BASELINE_8BIT => zune_jpeg::decode_color(image_pixel_module, data),

    #[cfg(feature = "native")]
    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::decode_color(image_pixel_module, data)
    }

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_color(image_pixel_module, data)
    }

    #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_color(image_pixel_module, data)
    }

    #[cfg(feature = "native")]
    &JPEG_2000 | &JPEG_2000_LOSSLESS_ONLY => {
      openjpeg::decode_color(image_pixel_module, data)
    }

    #[cfg(feature = "native")]
    &HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2000 => {
      #[cfg(not(target_arch = "wasm32"))]
      if decode_config.high_throughput_jpeg_2000_decoder
        == HighThroughputJpeg2000Decoder::OpenJph
      {
        return openjph::decode_color(image_pixel_module, data);
      }

      if decode_config.high_throughput_jpeg_2000_decoder
        == HighThroughputJpeg2000Decoder::OpenJpeg
      {
        return openjpeg::decode_color(image_pixel_module, data);
      }

      Err(PixelDataDecodeError::DecoderNotAvailable {
        name: decode_config.high_throughput_jpeg_2000_decoder.to_string(),
      })
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      #[cfg(all(feature = "native", not(target_arch = "wasm32")))]
      if decode_config.jpeg_xl_decoder == JpegXlDecoder::LibJxl {
        return libjxl::decode_color(image_pixel_module, data);
      }

      if decode_config.jpeg_xl_decoder == JpegXlDecoder::JxlOxide {
        return jxl_oxide::decode_color(image_pixel_module, data);
      }

      Err(PixelDataDecodeError::DecoderNotAvailable {
        name: decode_config.jpeg_xl_decoder.to_string(),
      })
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => native::decode_color(
      image_pixel_module,
      &inflate_frame_data(data, image_pixel_module)?,
    ),

    _ => {
      Err(PixelDataDecodeError::TransferSyntaxNotSupported { transfer_syntax })
    }
  }
}

/// Inflates deflated data for a single frame. This is used by the 'Deflated
/// Image Frame Compression' transfer syntax.
///
fn inflate_frame_data(
  data: &[u8],
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, PixelDataDecodeError> {
  let mut decompressor = flate2::Decompress::new(false);
  let mut inflated_data = vec![0; image_pixel_module.frame_size_in_bytes()];

  match decompressor.decompress(
    data,
    &mut inflated_data,
    flate2::FlushDecompress::Finish,
  ) {
    Ok(status) => {
      if status != flate2::Status::StreamEnd {
        return Err(PixelDataDecodeError::DataInvalid {
          details: "Frame data inflate did not reach the end of the stream"
            .to_string(),
        });
      }

      if decompressor.total_out() != inflated_data.len() as u64 {
        return Err(PixelDataDecodeError::DataInvalid {
          details: format!(
            "Frame data inflate produced {} bytes but {} bytes were expected",
            decompressor.total_out(),
            inflated_data.len()
          ),
        });
      }

      Ok(inflated_data)
    }

    Err(e) => Err(PixelDataDecodeError::DataInvalid {
      details: format!("Frame data inflate failed with '{e}'"),
    }),
  }
}
