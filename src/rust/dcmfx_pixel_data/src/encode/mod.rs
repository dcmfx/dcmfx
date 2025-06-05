#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{DcmfxError, TransferSyntax, transfer_syntax};

use crate::{
  ColorImage, ColorSpace, MonochromeImage, PixelDataFrame,
  iods::image_pixel_module::{BitsAllocated, ImagePixelModule},
};

mod charls;
mod jpeg_2000;
mod jpeg_encoder;
mod libjpeg_12bit;
mod native;
mod openjpeg;
mod openjph;
mod rle_lossless;

/// Configuration used when encoding pixel data.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelDataEncodeConfig {
  quality: u8,
  zlib_compression_level: u32,
}

impl PixelDataEncodeConfig {
  /// Creates a new encode config with default values.
  ///
  pub fn new() -> Self {
    PixelDataEncodeConfig {
      quality: 85,
      zlib_compression_level: 6,
    }
  }

  /// Returns the quality to use when lossy compressing pixel data.
  ///
  /// The value ranges from 1 (lowest quality), through to 100 (highest
  /// quality). It is used by the following transfer syntaxes:
  ///
  /// - JPEG Baseline 8-bit
  /// - JPEG Extended 12-bit
  /// - JPEG 2000 (Lossy)
  /// - High-Throughput JPEG 2000 (Lossy)
  ///
  /// Default: 85.
  ///
  pub fn quality(&self) -> u8 {
    self.quality
  }

  /// Sets the quality to use when performing lossy image compression of pixel
  /// data, e.g. in the JPEG (Process 1) transfer syntax.
  ///
  pub fn set_quality(&mut self, quality: u8) {
    self.quality = quality.clamp(1, 100);
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

  /// Sets the zlib compression level used when encoding pixel data into the
  /// 'Deflated Image Frame Compression' transfer syntax.
  ///
  pub fn set_zlib_compression_level(&mut self, compression_level: u32) {
    self.zlib_compression_level = compression_level.clamp(0, 9);
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

  /// The specified [`ImagePixelModule`] is not supported for encoding into the
  /// requested transfer syntax. This error may be returned by
  /// [`encode_image_pixel_module()`].
  ImagePixelModuleNotSupported {
    image_pixel_module: Box<ImagePixelModule>,
    transfer_syntax: &'static TransferSyntax,
  },

  /// The [`MonochromeImage`] or [`ColorImage`] was not able to be encoded,
  /// either because the image's data was not compatible with the configuration
  /// of the [`ImagePixelModule`], or the configuration is valid but isn't
  /// supported by the encoder for the requested transfer syntax.
  NotSupported {
    image_pixel_module: Box<ImagePixelModule>,
    input_bits_allocated: BitsAllocated,
    input_color_space: Option<ColorSpace>,
  },

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
      Self::ImagePixelModuleNotSupported { .. } => {
        "Image Pixel Module not supported by encoder".to_string()
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
        write!(
          f,
          "Transfer syntax '{}' not supported",
          transfer_syntax.name
        )
      }

      Self::ImagePixelModuleNotSupported { .. } => {
        write!(f, "Image Pixel Module not supported by encoder",)
      }

      Self::NotSupported { .. } => {
        write!(f, "Image or pixel configuration not supported by encoder")
      }

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

      Self::ImagePixelModuleNotSupported {
        image_pixel_module,
        transfer_syntax,
      } => {
        lines.push(format!("  Image pixel module: {}", image_pixel_module));
        lines.push(format!("  Transfer syntax: {}", transfer_syntax.name));
      }

      Self::NotSupported {
        image_pixel_module,
        input_bits_allocated,
        input_color_space,
      } => {
        lines.push(format!("  Image pixel module: {}", image_pixel_module));
        lines.push(format!(
          "  Input bits allocated: {}",
          u8::from(*input_bits_allocated)
        ));

        if let Some(input_color_space) = input_color_space {
          lines.push(format!("  Input color space: {:?}", input_color_space));
        }
      }

      Self::OtherError { details, .. } => {
        lines.push(format!("  Details: {}", details));
      }
    }

    lines
  }
}

/// Returns the resulting Image Pixel Module following encoding into the
/// specified transfer syntax.
///
#[allow(clippy::result_unit_err)]
pub fn encode_image_pixel_module(
  image_pixel_module: ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
  encode_config: &PixelDataEncodeConfig,
) -> Result<ImagePixelModule, PixelDataEncodeError> {
  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN
    | &DEFLATED_IMAGE_FRAME_COMPRESSION => {
      native::encode_image_pixel_module(image_pixel_module.clone())
    }

    &RLE_LOSSLESS => {
      rle_lossless::encode_image_pixel_module(image_pixel_module.clone())
    }

    &JPEG_BASELINE_8BIT => {
      jpeg_encoder::encode_image_pixel_module(image_pixel_module.clone())
    }

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::encode_image_pixel_module(image_pixel_module.clone())
    }

    &JPEG_LS_LOSSLESS => {
      charls::encode_image_pixel_module(image_pixel_module.clone(), false)
    }

    &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::encode_image_pixel_module(image_pixel_module.clone(), true)
    }

    &JPEG_2K_LOSSLESS_ONLY => {
      jpeg_2000::encode_image_pixel_module(image_pixel_module.clone(), None)
    }

    &JPEG_2K => jpeg_2000::encode_image_pixel_module(
      image_pixel_module.clone(),
      Some(encode_config.quality),
    ),

    &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY => {
      jpeg_2000::encode_image_pixel_module(image_pixel_module.clone(), None)
    }

    &HIGH_THROUGHPUT_JPEG_2K => jpeg_2000::encode_image_pixel_module(
      image_pixel_module.clone(),
      Some(encode_config.quality),
    ),

    _ => {
      return Err(PixelDataEncodeError::TransferSyntaxNotSupported {
        transfer_syntax,
      });
    }
  }
  .map_err(|_| PixelDataEncodeError::ImagePixelModuleNotSupported {
    image_pixel_module: Box::new(image_pixel_module.clone()),
    transfer_syntax,
  })
}

/// Encodes a [`MonochromeImage`] into raw pixel data bytes.
///
pub fn encode_monochrome(
  image: &MonochromeImage,
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
  encode_config: &PixelDataEncodeConfig,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => {
      native::encode_monochrome(image, image_pixel_module)
    }

    &RLE_LOSSLESS => rle_lossless::encode_monochrome(image, image_pixel_module)
      .map(PixelDataFrame::new_from_bytes),

    &JPEG_BASELINE_8BIT => {
      jpeg_encoder::encode_monochrome(image, image_pixel_module, encode_config)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::encode_monochrome(image, image_pixel_module, encode_config)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_LS_LOSSLESS => {
      charls::encode_monochrome(image, image_pixel_module, false)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::encode_monochrome(image, image_pixel_module, true)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_2K_LOSSLESS_ONLY => {
      openjpeg::encode_monochrome(image, image_pixel_module, None)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_2K => openjpeg::encode_monochrome(
      image,
      image_pixel_module,
      Some(encode_config.quality),
    )
    .map(PixelDataFrame::new_from_bytes),

    &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY => {
      openjph::encode_monochrome(image, image_pixel_module, None)
        .map(PixelDataFrame::new_from_bytes)
    }

    &HIGH_THROUGHPUT_JPEG_2K => openjph::encode_monochrome(
      image,
      image_pixel_module,
      Some(encode_config.quality),
    )
    .map(PixelDataFrame::new_from_bytes),

    &DEFLATED_IMAGE_FRAME_COMPRESSION => deflate_frame_data(
      native::encode_monochrome(image, image_pixel_module)?,
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
  image_pixel_module: &ImagePixelModule,
  transfer_syntax: &'static TransferSyntax,
  encode_config: &PixelDataEncodeConfig,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => {
      native::encode_color(image, image_pixel_module)
        .map(PixelDataFrame::new_from_bytes)
    }

    &RLE_LOSSLESS => rle_lossless::encode_color(image, image_pixel_module)
      .map(PixelDataFrame::new_from_bytes),

    &JPEG_BASELINE_8BIT => {
      jpeg_encoder::encode_color(image, image_pixel_module, encode_config)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::encode_color(image, image_pixel_module, encode_config)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_LS_LOSSLESS => charls::encode_color(image, image_pixel_module, false)
      .map(PixelDataFrame::new_from_bytes),

    &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::encode_color(image, image_pixel_module, true)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_2K_LOSSLESS_ONLY => {
      openjpeg::encode_color(image, image_pixel_module, None)
        .map(PixelDataFrame::new_from_bytes)
    }

    &JPEG_2K => openjpeg::encode_color(
      image,
      image_pixel_module,
      Some(encode_config.quality),
    )
    .map(PixelDataFrame::new_from_bytes),

    &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY => {
      openjph::encode_color(image, image_pixel_module, None)
        .map(PixelDataFrame::new_from_bytes)
    }

    &HIGH_THROUGHPUT_JPEG_2K => openjph::encode_color(
      image,
      image_pixel_module,
      Some(encode_config.quality),
    )
    .map(PixelDataFrame::new_from_bytes),

    &DEFLATED_IMAGE_FRAME_COMPRESSION => {
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
