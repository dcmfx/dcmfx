//! Access pixel data in a DICOM data set.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

#[cfg(not(feature = "std"))]
mod no_std_allocator;

mod color_image;
pub mod decode;
pub mod encode;
mod grayscale_pipeline;
pub mod iods;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
mod jpeg_xl_jpeg_recompression;
mod lookup_table;
mod monochrome_image;
mod pixel_data_frame;
mod pixel_data_renderer;
pub mod standard_color_palettes;
mod stored_value_output_cache;
pub mod transforms;
mod utils;

pub use color_image::{ColorImage, ColorSpace};
pub use decode::{PixelDataDecodeConfig, PixelDataDecodeError};
pub use encode::{PixelDataEncodeConfig, PixelDataEncodeError};
pub use grayscale_pipeline::GrayscalePipeline;
pub use lookup_table::LookupTable;
pub use monochrome_image::MonochromeImage;
pub use pixel_data_frame::PixelDataFrame;
pub use pixel_data_renderer::PixelDataRenderer;
pub use standard_color_palettes::StandardColorPalette;
pub use stored_value_output_cache::StoredValueOutputCache;

use transforms::{
  P10PixelDataFrameTransform, P10PixelDataFrameTransformError,
  P10PixelDataTranscodeTransform, P10PixelDataTranscodeTransformError,
  TranscodeImageDataFunctions,
};

use dcmfx_core::{
  DataError, DataSet, IodModule, TransferSyntax, dictionary, transfer_syntax,
};
use dcmfx_p10::{DataSetBuilder, DataSetP10Extensions};

/// Adds functions to [`DataSet`] for working with pixel data it contains.
///
pub trait DataSetPixelDataExtensions
where
  Self: Sized,
{
  /// Returns the frames of pixel data in this data set in their raw form.
  ///
  /// The *'(7FE0,0010) Pixel Data'* data element must be present in the data
  /// set, and the *'(0028,0008) Number of Frames'*, *'(7FE0,0001) Extended
  /// Offset Table'*, and *'(7FE0,0002) Extended Offset Table Lengths'* data
  /// elements are used when present and relevant.
  ///
  fn get_pixel_data_frames(
    &self,
  ) -> Result<Vec<PixelDataFrame>, P10PixelDataFrameTransformError>;

  /// Returns the frames of pixel data in this data set as fully resolved RGB8
  /// images. Output image data may have been subjected to a lossy conversion to
  /// 8-bit depth.
  ///
  /// For monochrome frames, any Modality LUT and VOI LUT present in the data
  /// set are applied to reach a final grayscale value, which is duplicated
  /// across the RGB components.
  ///
  /// Grayscale values can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::standard_color_palettes`].
  ///
  fn get_pixel_data_images(
    &self,
    color_palette: Option<&StandardColorPalette>,
  ) -> Result<Vec<image::RgbImage>, GetPixelDataError>;

  /// Returns the frames of pixel data in this data set as [`MonochromeImage`]s.
  ///
  /// This will only succeed when the pixel data uses a monochrome photometric
  /// interpretation. Returned images needs to have a grayscale pipeline applied
  /// in order to reach final grayscale display values.
  ///
  fn get_pixel_data_monochrome_images(
    &self,
  ) -> Result<Vec<MonochromeImage>, GetPixelDataError>;

  /// Returns the frames of pixel data in this data set as [`ColorImage`]s.
  ///
  /// This will only succeed when the pixel data has three channels.
  ///
  fn get_pixel_data_color_images(
    &self,
  ) -> Result<Vec<ColorImage>, GetPixelDataError>;

  /// Transcode's the pixel data in this data set into a new data set that uses
  /// the specified [`TransferSyntax`]. If this data set does not contain a
  /// valid Image Pixel Module then no transcoding will occur and `Ok(None)` is
  /// returned.
  ///
  fn transcode_pixel_data(
    &self,
    target_transfer_syntax: &'static TransferSyntax,
    decode_config: PixelDataDecodeConfig,
    encode_config: PixelDataEncodeConfig,
    image_data_functions: Option<TranscodeImageDataFunctions>,
  ) -> Result<Option<DataSet>, P10PixelDataTranscodeTransformError>;
}

impl DataSetPixelDataExtensions for DataSet {
  fn get_pixel_data_frames(
    &self,
  ) -> Result<Vec<PixelDataFrame>, P10PixelDataFrameTransformError> {
    // Create a new data set containing only the data elements needed by the
    // pixel data frame transform. This avoids calling DataSet::to_p10_tokens()
    // on the whole data set.
    let mut ds = DataSet::new();
    for tag in [
      dictionary::NUMBER_OF_FRAMES.tag,
      dictionary::ROWS.tag,
      dictionary::COLUMNS.tag,
      dictionary::BITS_ALLOCATED.tag,
      dictionary::EXTENDED_OFFSET_TABLE.tag,
      dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
      dictionary::PIXEL_DATA.tag,
    ] {
      if let Ok(value) = self.get_value(tag) {
        ds.insert(tag, value.clone());
      }
    }

    // Pass the cut down data set through a pixel data filter and collect all
    // emitted frames
    let mut pixel_data_frame_transform = P10PixelDataFrameTransform::new();
    let mut frames = vec![];
    ds.to_p10_tokens(&mut |token| {
      frames.extend_from_slice(&pixel_data_frame_transform.add_token(token)?);
      Ok(())
    })?;

    Ok(frames)
  }

  fn get_pixel_data_images(
    &self,
    color_palette: Option<&StandardColorPalette>,
  ) -> Result<Vec<image::RgbImage>, GetPixelDataError> {
    get_pixel_data(self, |renderer, frame| {
      renderer.render_frame(frame, color_palette)
    })
  }

  fn get_pixel_data_monochrome_images(
    &self,
  ) -> Result<Vec<MonochromeImage>, GetPixelDataError> {
    get_pixel_data(self, |renderer, frame| {
      renderer.decode_monochrome_frame(frame)
    })
  }

  fn get_pixel_data_color_images(
    &self,
  ) -> Result<Vec<ColorImage>, GetPixelDataError> {
    get_pixel_data(self, |renderer, frame| renderer.decode_color_frame(frame))
  }

  fn transcode_pixel_data(
    &self,
    output_transfer_syntax: &'static TransferSyntax,
    decode_config: PixelDataDecodeConfig,
    encode_config: PixelDataEncodeConfig,
    image_data_functions: Option<TranscodeImageDataFunctions>,
  ) -> Result<Option<DataSet>, P10PixelDataTranscodeTransformError> {
    let mut transcode_transform = P10PixelDataTranscodeTransform::new(
      output_transfer_syntax,
      decode_config,
      encode_config,
      image_data_functions,
    );

    let mut data_set_builder = DataSetBuilder::new();

    self.to_p10_tokens(&mut |token| {
      let tokens = transcode_transform.add_token(token)?;

      for token in tokens.iter() {
        data_set_builder
          .add_token(token)
          .map_err(P10PixelDataTranscodeTransformError::P10Error)?;
      }

      Ok(())
    })?;

    if !transcode_transform.is_active() {
      return Ok(None);
    }

    Ok(Some(data_set_builder.final_data_set().unwrap()))
  }
}

/// An error that occurred getting pixel data using one of the functions in the
/// [`DataSetPixelDataExtensions`] trait.
///
#[derive(Clone, Debug, PartialEq)]
pub enum GetPixelDataError {
  /// An error that occurred when reading the pixel data renderer from the data
  /// elements from the stream of DICOM P10 tokens.
  DataError(DataError),

  /// An error that occurred when reading the raw frames of pixel data from the
  /// stream of DICOM P10 tokens.
  P10PixelDataFrameTransformError(P10PixelDataFrameTransformError),

  /// An error that occurred when decoding a raw frame of pixel data.
  PixelDataDecodeError {
    frame_index: usize,
    error: PixelDataDecodeError,
  },
}

fn get_pixel_data<T, F>(
  data_set: &DataSet,
  mut process_frame: F,
) -> Result<Vec<T>, GetPixelDataError>
where
  F: FnMut(
    &PixelDataRenderer,
    &mut PixelDataFrame,
  ) -> Result<T, PixelDataDecodeError>,
{
  let renderer = PixelDataRenderer::from_data_set(data_set)
    .map_err(GetPixelDataError::DataError)?;

  let frames = data_set
    .get_pixel_data_frames()
    .map_err(GetPixelDataError::P10PixelDataFrameTransformError)?;

  frames
    .into_iter()
    .map(|mut frame| {
      process_frame(&renderer, &mut frame).map_err(|error| {
        GetPixelDataError::PixelDataDecodeError {
          frame_index: frame.index().unwrap(),
          error,
        }
      })
    })
    .collect()
}

/// Returns the file extension to use for pixel data in the given transfer
/// syntax. If there is no sensible file extension to use then `".bin"` is
/// returned.
///
pub fn file_extension_for_transfer_syntax(ts: &TransferSyntax) -> &'static str {
  use transfer_syntax::*;

  match ts {
    // JPEG and JPEG Lossless use the .jpg extension
    ts if ts == &JPEG_BASELINE_8BIT
      || ts == &JPEG_EXTENDED_12BIT
      || ts == &JPEG_LOSSLESS_NON_HIERARCHICAL
      || ts == &JPEG_LOSSLESS_NON_HIERARCHICAL_SV1 =>
    {
      ".jpg"
    }

    // JPEG-LS uses the .jls extension
    ts if ts == &JPEG_LS_LOSSLESS || ts == &JPEG_LS_LOSSY_NEAR_LOSSLESS => {
      ".jls"
    }

    // JPEG 2000 uses the .j2k extension
    ts if ts == &JPEG_2000_LOSSLESS_ONLY
      || ts == &JPEG_2000
      || ts == &JPEG_2000_MULTI_COMPONENT_LOSSLESS_ONLY
      || ts == &JPEG_2000_MULTI_COMPONENT =>
    {
      ".j2k"
    }

    // MPEG-2 uses the .mp2 extension
    ts if ts == &MPEG2_MAIN_PROFILE_MAIN_LEVEL
      || ts == &FRAGMENTABLE_MPEG2_MAIN_PROFILE_MAIN_LEVEL
      || ts == &MPEG2_MAIN_PROFILE_HIGH_LEVEL
      || ts == &FRAGMENTABLE_MPEG2_MAIN_PROFILE_HIGH_LEVEL =>
    {
      ".mp2"
    }

    // MPEG-4 uses the .mp4 extension
    ts if ts == &MPEG4_AVC_H264_HIGH_PROFILE
      || ts == &FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE
      || ts == &MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE
      || ts == &FRAGMENTABLE_MPEG4_AVC_H264_BD_COMPATIBLE_HIGH_PROFILE
      || ts == &MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO
      || ts == &FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_2D_VIDEO
      || ts == &MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO
      || ts == &FRAGMENTABLE_MPEG4_AVC_H264_HIGH_PROFILE_FOR_3D_VIDEO
      || ts == &MPEG4_AVC_H264_STEREO_HIGH_PROFILE
      || ts == &FRAGMENTABLE_MPEG4_AVC_H264_STEREO_HIGH_PROFILE =>
    {
      ".mp4"
    }

    // HEVC/H.265 also uses the .mp4 extension
    ts if ts == &HEVC_H265_MAIN_PROFILE || ts == &HEVC_H265_MAIN_10_PROFILE => {
      ".mp4"
    }

    // JPEG XL uses the .jxl extension
    ts if ts == &JPEG_XL_LOSSLESS
      || ts == &JPEG_XL_JPEG_RECOMPRESSION
      || ts == &JPEG_XL =>
    {
      ".jxl"
    }

    // High-Throughput JPEG 2000 uses the .jph extension
    ts if ts == &HIGH_THROUGHPUT_JPEG_2000_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2000_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2000 =>
    {
      ".jph"
    }

    // Deflated Image Frame Compression uses the .zz extension
    ts if ts == &DEFLATED_IMAGE_FRAME_COMPRESSION => ".zz",

    // Everything else uses the .bin extension as there isn't a more meaningful
    // image extension to use
    _ => ".bin",
  }
}

#[cfg(test)]
mod tests {
  #[cfg(not(feature = "std"))]
  use alloc::string::ToString;

  use super::*;
  use dcmfx_core::{
    DataElementValue, RcByteSlice, ValueRepresentation, dictionary,
  };

  #[test]
  fn read_native_empty_frame() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::ROWS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        RcByteSlice::empty(),
      )
      .unwrap(),
    );

    assert_eq!(ds.get_pixel_data_frames().unwrap(), vec![],);
  }

  #[test]
  fn read_native_single_frame() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::ROWS, &[2]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[2]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        vec![1, 2, 3, 4].into(),
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data_frames().unwrap(),
      vec![frame_with_fragments(&[&[1, 2, 3, 4]])],
    );
  }

  #[test]
  fn read_native_multi_frame() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[2])
      .unwrap();
    ds.insert_int_value(&dictionary::ROWS, &[1]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[2]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        vec![1, 2, 3, 4].into(),
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data_frames().unwrap(),
      vec![
        frame_with_fragments(&[&[1, 2]]),
        frame_with_fragments(&[&[3, 4]])
      ],
    );
  }

  #[test]
  fn read_native_multi_frame_with_one_bit_allocated() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[3])
      .unwrap();
    ds.insert_int_value(&dictionary::ROWS, &[3]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[5]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[1])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        vec![
          0b00110001, 0b00011011, 0b10100011, 0b01100101, 0b00010101,
          0b00000110,
        ]
        .into(),
      )
      .unwrap(),
    );

    let frames = ds.get_pixel_data_frames().unwrap();
    assert_eq!(*frames[0].to_bytes(), vec![0b00110001, 0b00011011]);
    assert_eq!(*frames[1].to_bytes(), vec![0b01000110, 0b11001011, 0]);
    assert_eq!(*frames[2].to_bytes(), vec![0b01010101, 0b00011000, 0]);
    assert_eq!(frames[0].bit_offset(), 0);
    assert_eq!(frames[1].bit_offset(), 7);
    assert_eq!(frames[2].bit_offset(), 6);
  }

  #[test]
  fn read_native_multi_frame_with_one_bit_allocated_and_multiple_frames_in_one_byte()
   {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[4])
      .unwrap();
    ds.insert_int_value(&dictionary::ROWS, &[1]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[3]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[1])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        vec![0b01010001, 0b00001101].into(),
      )
      .unwrap(),
    );

    let frames = ds.get_pixel_data_frames().unwrap();
    assert_eq!(frames.len(), 4);
    assert_eq!(frames[0].to_bytes()[0] & 7, 0b001);
    assert_eq!(frames[1].to_bytes()[0] & 7, 0b010);
    assert_eq!(frames[2].to_bytes()[0] & 7, 0b101);
    assert_eq!(frames[3].to_bytes()[0] & 7, 0b110);
    assert_eq!(frames[0].bit_offset(), 0);
    assert_eq!(frames[1].bit_offset(), 3);
    assert_eq!(frames[2].bit_offset(), 6);
    assert_eq!(frames[3].bit_offset(), 1);
  }

  #[test]
  fn read_native_multi_frame_malformed() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[3])
      .unwrap();
    ds.insert_int_value(&dictionary::ROWS, &[1]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[1]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();

    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        vec![1, 2, 3, 4].into(),
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data_frames().unwrap_err().to_string(),
      "DICOM Data Error: Invalid value at <unknown>, details: Multi-frame \
       pixel data of length 4 bytes does not divide evenly into 3 frames",
    );
  }

  // This test is taken from the DICOM standard. Ref: PS3.5 Table A.4-1.
  #[test]
  fn read_encapsulated_multiple_fragments_into_single_frame() {
    assert_eq!(
      data_set_with_three_fragments()
        .get_pixel_data_frames()
        .unwrap(),
      vec![frame_with_fragments(&[
        "1".repeat(0x4C6).as_bytes().to_vec().as_slice(),
        "2".repeat(0x24A).as_bytes().to_vec().as_slice(),
        "3".repeat(0x628).as_bytes().to_vec().as_slice()
      ])],
    );
  }

  #[test]
  fn read_encapsulated_multiple_fragments_into_multiple_frames() {
    let mut ds = data_set_with_three_fragments();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[3])
      .unwrap();

    assert_eq!(
      ds.get_pixel_data_frames().unwrap(),
      vec![
        frame_with_fragments(&["1"
          .repeat(0x4C6)
          .as_bytes()
          .to_vec()
          .as_slice()]),
        frame_with_fragments(&["2"
          .repeat(0x24A)
          .as_bytes()
          .to_vec()
          .as_slice()]),
        frame_with_fragments(&["3"
          .repeat(0x628)
          .as_bytes()
          .to_vec()
          .as_slice()]),
      ]
    );
  }

  // This test is taken from the DICOM standard. Ref: PS3.5 Table A.4-2.
  #[test]
  fn read_encapsulated_using_basic_offset_table() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::ROWS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          vec![0, 0, 0, 0, 0x46, 0x06, 0, 0].into(),
          "1".repeat(0x2C8).as_bytes().to_vec().into(),
          "2".repeat(0x36E).as_bytes().to_vec().into(),
          "3".repeat(0xBC8).as_bytes().to_vec().into(),
        ],
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data_frames().unwrap(),
      vec![
        frame_with_fragments(&[
          "1".repeat(0x2C8).as_bytes().to_vec().as_slice(),
          "2".repeat(0x36E).as_bytes().to_vec().as_slice()
        ]),
        frame_with_fragments(&["3"
          .repeat(0xBC8)
          .as_bytes()
          .to_vec()
          .as_slice()]),
      ]
    );
  }

  #[test]
  fn read_encapsulated_using_extended_offset_table() {
    let mut ds = data_set_with_three_fragments();
    ds.insert(
      dictionary::EXTENDED_OFFSET_TABLE.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        vec![
          0, 0, 0, 0, 0, 0, 0, 0, 206, 4, 0, 0, 0, 0, 0, 0, 32, 7, 0, 0, 0, 0,
          0, 0,
        ]
        .into(),
      )
      .unwrap(),
    );
    ds.insert(
      dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        vec![
          198, 4, 0, 0, 0, 0, 0, 0, 74, 2, 0, 0, 0, 0, 0, 0, 39, 6, 0, 0, 0, 0,
          0, 0,
        ]
        .into(),
      )
      .unwrap(),
    );

    assert_eq!(
      ds.get_pixel_data_frames().unwrap(),
      vec![
        frame_with_fragments(&["1"
          .repeat(0x4C6)
          .as_bytes()
          .to_vec()
          .as_slice()]),
        frame_with_fragments(&["2"
          .repeat(0x24A)
          .as_bytes()
          .to_vec()
          .as_slice()]),
        frame_with_fragments(&["3"
          .repeat(0x627)
          .as_bytes()
          .to_vec()
          .as_slice()]),
      ],
    );
  }

  fn frame_with_fragments(fragments: &[&[u8]]) -> PixelDataFrame {
    let mut frame = PixelDataFrame::new();

    for fragment in fragments.iter() {
      frame.push_bytes(fragment.to_vec().into());
    }

    frame
  }

  fn data_set_with_three_fragments() -> DataSet {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::ROWS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          RcByteSlice::empty(),
          "1".repeat(0x4C6).as_bytes().to_vec().into(),
          "2".repeat(0x24A).as_bytes().to_vec().into(),
          "3".repeat(0x628).as_bytes().to_vec().into(),
        ],
      )
      .unwrap(),
    );
    ds
  }
}
