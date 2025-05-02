#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::{DataError, DcmfxError, TransferSyntax, transfer_syntax};

use crate::{iods::ImagePixelModule, ColorImage, MonochromeImage, PixelDataFrame};

#[cfg(not(target_arch = "wasm32"))]
mod charls;
mod jpeg_decoder;
mod jxl_oxide;
mod libjpeg_12bit;
mod native;
mod openjpeg;
mod rle_lossless;
mod zune_jpeg;

/// Errors that can occur when decoding frames of image data in a specific
/// transfer syntax.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PixelDataDecodeError {
  /// The transfer syntax is not supported for decoding.
  TransferSyntaxNotSupported {
    transfer_syntax: &'static TransferSyntax,
  },

  /// The pixel data configuration is not supported for decoding from the source
  /// transfer syntax.
  NotSupported { details: String },
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
      Self::NotSupported { .. } => "Configuration not supported".to_string(),
    }
  }
}

impl core::fmt::Display for PixelDataDecodeError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::TransferSyntaxNotSupported { transfer_syntax } => {
        write!(f, "Transfer syntax not supported: {}", transfer_syntax.name)
      }

      Self::NotSupported { details } => write!(
        f,
        "Pixel data configuration not supported, details: {}",
        details
      ),
    }
  }
}

impl DcmfxError for PixelDataDecodeError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("Pixel data decode error {}", task_description),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match self {
      Self::TransferSyntaxNotSupported { transfer_syntax } => {
        lines.push(format!("  Transfer syntax: {}", transfer_syntax.name));
      }
      Self::NotSupported { details } => {
        lines.push(format!("  Details: {}", details));
      }
    }

    lines
  }
}

/// Decodes a frame of monochrome pixel data into a [`MonochromeImage`]. The
/// returned image needs to have a grayscale pipeline applied in order to reach
/// final grayscale display values.
///
pub fn decode_monochrome(
  frame: &mut PixelDataFrame,
  transfer_syntax: &TransferSyntax,
  image_pixel_module: &ImagePixelModule,
) -> Result<MonochromeImage, DataError> {
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

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::decode_monochrome(image_pixel_module, data)
    }

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_monochrome(image_pixel_module, data)
    }

    &JPEG_2K
    | &JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K
    | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
      openjpeg::decode_monochrome(image_pixel_module, data)
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      jxl_oxide::decode_monochrome(image_pixel_module, data)
    }

    #[cfg(not(target_arch = "wasm32"))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_monochrome(image_pixel_module, data)
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => native::decode_monochrome(
      image_pixel_module,
      &inflate_frame_data(data, image_pixel_module)?,
      0,
    ),

    _ => Err(DataError::new_value_unsupported(format!(
      "Transfer syntax '{}' is not able to be decoded",
      transfer_syntax.name,
    ))),
  }
}

/// Decodes a frame of color pixel data into a [`ColorImage`].
///
pub fn decode_color(
  frame: &mut PixelDataFrame,
  transfer_syntax: &TransferSyntax,
  image_pixel_module: &ImagePixelModule,
) -> Result<ColorImage, DataError> {
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

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::decode_color(image_pixel_module, data)
    }

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_color(image_pixel_module, data)
    }

    &JPEG_2K
    | &JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K
    | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
      openjpeg::decode_color(image_pixel_module, data)
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      jxl_oxide::decode_color(image_pixel_module, data)
    }

    #[cfg(not(target_arch = "wasm32"))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_color(image_pixel_module, data)
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => native::decode_color(
      image_pixel_module,
      &inflate_frame_data(data, image_pixel_module)?,
    ),

    _ => Err(DataError::new_value_unsupported(format!(
      "Transfer syntax '{}' is not able to be decoded",
      transfer_syntax.name
    ))),
  }
}

/// Inflates deflated data for a single frame. This is used by the 'Deflated
/// Image Frame Compression' transfer syntax.
///
fn inflate_frame_data(
  data: &[u8],
  image_pixel_module: &ImagePixelModule,
) -> Result<Vec<u8>, DataError> {
  let mut decompressor = flate2::Decompress::new(false);
  let mut inflated_data = vec![0; image_pixel_module.frame_size_in_bytes()];

  match decompressor.decompress(
    data,
    &mut inflated_data,
    flate2::FlushDecompress::Finish,
  ) {
    Ok(status) => {
      if status != flate2::Status::StreamEnd {
        return Err(DataError::new_value_invalid(
          "Frame data inflate did not reach the end of the stream".to_string(),
        ));
      }

      if decompressor.total_out() != inflated_data.len() as u64 {
        return Err(DataError::new_value_invalid(format!(
          "Frame data inflate produced {} bytes but {} bytes were expected",
          decompressor.total_out(),
          inflated_data.len()
        )));
      }

      Ok(inflated_data)
    }

    Err(_) => Err(DataError::new_value_invalid(
      "Frame data inflate failed".to_string(),
    )),
  }
}
