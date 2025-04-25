#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec, vec::Vec};

use dcmfx_core::{DataError, TransferSyntax, transfer_syntax};

use crate::{
  ColorImage, PixelDataFrame, SingleChannelImage, iods::ImagePixelModule,
};

#[cfg(not(target_arch = "wasm32"))]
mod charls;
mod jpeg_decoder;
mod jxl_oxide;
mod libjpeg_12bit;
mod native;
mod openjpeg;
mod rle_lossless;
mod zune_jpeg;

/// Decodes a frame of single channel pixel data into a [`SingleChannelImage`].
/// The returned image needs to have a grayscale pipeline applied in order to
/// reach final grayscale display values.
///
pub fn decode_single_channel(
  frame: &mut PixelDataFrame,
  transfer_syntax: &TransferSyntax,
  image_pixel_module: &ImagePixelModule,
) -> Result<SingleChannelImage, DataError> {
  let frame_bit_offset = frame.bit_offset();
  let data = frame.combine_fragments();

  use transfer_syntax::*;

  match transfer_syntax {
    &IMPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_LITTLE_ENDIAN
    | &ENCAPSULATED_UNCOMPRESSED_EXPLICIT_VR_LITTLE_ENDIAN
    | &DEFLATED_EXPLICIT_VR_LITTLE_ENDIAN
    | &EXPLICIT_VR_BIG_ENDIAN => {
      native::decode_single_channel(image_pixel_module, data, frame_bit_offset)
    }

    &RLE_LOSSLESS => {
      rle_lossless::decode_single_channel(image_pixel_module, data)
    }

    &JPEG_BASELINE_8BIT => {
      zune_jpeg::decode_single_channel(image_pixel_module, data)
    }

    &JPEG_EXTENDED_12BIT => {
      libjpeg_12bit::decode_single_channel(image_pixel_module, data)
    }

    &JPEG_LOSSLESS_NON_HIERARCHICAL | &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 => {
      jpeg_decoder::decode_single_channel(image_pixel_module, data)
    }

    &JPEG_2K
    | &JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K
    | &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
    | &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY => {
      openjpeg::decode_single_channel(image_pixel_module, data)
    }

    &JPEG_XL_LOSSLESS | &JPEG_XL_JPEG_RECOMPRESSION | &JPEG_XL => {
      jxl_oxide::decode_single_channel(image_pixel_module, data)
    }

    #[cfg(not(target_arch = "wasm32"))]
    &JPEG_LS_LOSSLESS | &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      charls::decode_single_channel(image_pixel_module, data)
    }

    &DEFLATED_IMAGE_FRAME_COMPRESSION => native::decode_single_channel(
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
  let data = frame.combine_fragments();

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
