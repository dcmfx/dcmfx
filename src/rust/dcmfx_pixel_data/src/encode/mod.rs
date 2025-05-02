#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec,
};

use dcmfx_core::{DcmfxError, TransferSyntax, transfer_syntax};

use crate::{
  ColorImage, MonochromeImage, PixelDataFrame, iods::ImagePixelModule,
};

mod native;
mod rle_lossless;

/// Configuration used when encoding pixel data.
///
#[derive(Clone, Debug, PartialEq)]
pub struct PixelDataEncodeConfig {
  zlib_compression_level: u32,
}

impl PixelDataEncodeConfig {
  /// Creates a new encode config with default values.
  ///
  pub fn new() -> Self {
    PixelDataEncodeConfig {
      zlib_compression_level: 6,
    }
  }

  /// Returns the zlib compression level used when encoding pixel data into the
  /// 'Deflated Image Frame Compression' transfer syntax.
  ///
  /// The level ranges from 0, meaning no compression, through to 9, which gives
  /// the best compression at the cost of speed.
  ///
  /// Default: 6.
  ///
  pub fn zlib_compression_level(&self) -> u32 {
    self.zlib_compression_level
  }

  /// Creates a new encode config with default values.
  ///
  pub fn set_zlib_compression_level(&mut self, compression_level: u32) {
    self.zlib_compression_level = compression_level.clamp(1, 9);
  }
}

impl Default for PixelDataEncodeConfig {
  fn default() -> Self {
    Self::new()
  }
}

/// Errors that can occur when encoding frames of image data into a specific
/// transfer syntax.
///
#[derive(Clone, Debug, PartialEq)]
pub enum PixelDataEncodeError {
  /// The target transfer syntax is not supported for encoding.
  TransferSyntaxNotSupported {
    transfer_syntax: &'static TransferSyntax,
  },

  /// The pixel data configuration is not supported for encoding into the target
  /// transfer syntax.
  NotSupported { details: String },

  /// There was another untyped error when encoding pixel data.
  OtherError { name: String, details: String },
}

impl PixelDataEncodeError {
  /// Returns the name of the pixel data encode error as a human-readable
  /// string.
  ///
  pub fn name(&self) -> String {
    match self {
      Self::TransferSyntaxNotSupported { .. } => {
        "Transfer syntax not supported".to_string()
      }
      Self::NotSupported { .. } => "Configuration not supported".to_string(),
      Self::OtherError { name, .. } => name.to_string(),
    }
  }
}

impl core::fmt::Display for PixelDataEncodeError {
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

      Self::OtherError { name, details } => {
        write!(f, "{}, details: {}", name, details)
      }
    }
  }
}

impl DcmfxError for PixelDataEncodeError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("Pixel data encode error {}", task_description),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match self {
      Self::TransferSyntaxNotSupported { transfer_syntax } => {
        lines.push(format!("  Transfer syntax: {}", transfer_syntax.name));
      }
      Self::NotSupported { details } | Self::OtherError { details, .. } => {
        lines.push(format!("  Details: {}", details));
      }
    }

    lines
  }
}

/// Encodes a [`MonochromeImage`] into raw pixel data bytes.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  transfer_syntax: &'static TransferSyntax,
  encode_config: &PixelDataEncodeConfig,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  match transfer_syntax {
    &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN => {
      Ok(native::encode_monochrome(image))
    }

    &transfer_syntax::RLE_LOSSLESS => {
      rle_lossless::encode_monochrome(image).map(PixelDataFrame::new_from_bytes)
    }

    &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION => deflate_frame_data(
      native::encode_monochrome(image),
      encode_config.zlib_compression_level,
    ),

    _ => {
      Err(PixelDataEncodeError::TransferSyntaxNotSupported { transfer_syntax })
    }
  }
}

/// Encodes a [`ColorImage`] into raw pixel data bytes.
///
pub fn encode_color(
  image: &ColorImage,
  transfer_syntax: &'static TransferSyntax,
  image_pixel_module: &ImagePixelModule,
  encode_config: &PixelDataEncodeConfig,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  match transfer_syntax {
    &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN => {
      native::encode_color(image, image_pixel_module)
        .map(PixelDataFrame::new_from_bytes)
    }

    &transfer_syntax::RLE_LOSSLESS => {
      rle_lossless::encode_color(image, image_pixel_module)
        .map(PixelDataFrame::new_from_bytes)
    }

    &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION => {
      let frame = native::encode_color(image, image_pixel_module)
        .map(PixelDataFrame::new_from_bytes)?;

      deflate_frame_data(frame, encode_config.zlib_compression_level)
    }

    _ => {
      Err(PixelDataEncodeError::TransferSyntaxNotSupported { transfer_syntax })
    }
  }
}

/// Deflates raw data for a single frame. This is used by the 'Deflated Image
/// Frame Compression' transfer syntax.
///
fn deflate_frame_data(
  mut frame: PixelDataFrame,
  compression_level: u32,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  let mut input = frame.combine_chunks();

  let mut deflated_frame = PixelDataFrame::new();

  let compression_level = flate2::Compression::new(compression_level);
  let mut compressor = flate2::Compress::new(compression_level, false);

  // Loop around compressing into 256KiB output chunks until all data is
  // deflated
  loop {
    let flush = if input.is_empty() {
      flate2::FlushCompress::Finish
    } else {
      flate2::FlushCompress::None
    };

    let initial_total_in = compressor.total_in();
    let initial_total_out = compressor.total_out();

    let mut output_buffer = vec![0u8; 256 * 1024];
    let status = compressor
      .compress(input, &mut output_buffer, flush)
      .map_err(|e| PixelDataEncodeError::OtherError {
        name: "Deflate failed".to_string(),
        details: e.to_string(),
      })?;

    // Set size of the output chunk
    output_buffer
      .truncate((compressor.total_out() - initial_total_out) as usize);

    // Add the output buffer to the deflated frame if it has content
    if !output_buffer.is_empty() {
      output_buffer.shrink_to_fit();
      deflated_frame.push_bytes(output_buffer.into());
    }

    if status == flate2::Status::StreamEnd {
      return Ok(deflated_frame);
    }

    // Slice off input bytes that have been consumed by the compressor
    let input_bytes_read = (compressor.total_in() - initial_total_in) as usize;
    input = &input[input_bytes_read..];
  }
}
