#[cfg(not(feature = "std"))]
use alloc::{
  string::{String, ToString},
  vec,
};

use dcmfx_core::{TransferSyntax, transfer_syntax};

use crate::{
  ColorImage, PixelDataFrame, SingleChannelImage, iods::ImagePixelModule,
};

mod native;
mod rle_lossless;

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

  /// There was an error performing a deflate compression operation when
  /// compressing pixel data.
  DeflateError { details: String },

  /// There was an error performing a deflate compression operation when
  /// compressing pixel data.
  OtherError { details: String },
}

/// Encodes a [`SingleChannelImage`] into raw pixel data bytes.
///
pub fn encode_single_channel(
  image: &SingleChannelImage,
  transfer_syntax: &'static TransferSyntax,
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  match transfer_syntax {
    &transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &transfer_syntax::EXPLICIT_VR_BIG_ENDIAN => {
      Ok(native::encode_single_channel(image))
    }

    &transfer_syntax::RLE_LOSSLESS => {
      rle_lossless::encode_single_channel(image)
        .map(PixelDataFrame::new_from_bytes)
    }

    &transfer_syntax::DEFLATED_IMAGE_FRAME_COMPRESSION => {
      deflate_frame_data(native::encode_single_channel(image))
    }

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

      deflate_frame_data(frame)
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
) -> Result<PixelDataFrame, PixelDataEncodeError> {
  let mut input = frame.combine_fragments();

  let mut deflated_frame = PixelDataFrame::new();

  let compression_level = flate2::Compression::default();
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
      .map_err(|e| PixelDataEncodeError::DeflateError {
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
