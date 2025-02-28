//! Access pixel data in a DICOM data set.

mod luts;
mod pixel_data_definition;
mod pixel_data_filter;
mod pixel_data_frame;
mod pixel_data_native;
mod pixel_data_reader;

pub use luts::{
  ColorPalette, LookupTable, ModalityLut, StandardColorPalette, VoiLut,
  VoiLutFunction, VoiWindow,
};
pub use pixel_data_definition::{
  BitsAllocated, PhotometricInterpretation, PixelDataDefinition,
  PixelRepresentation, PlanarConfiguration, SamplesPerPixel,
};
pub use pixel_data_filter::{PixelDataFilter, PixelDataFilterError};
pub use pixel_data_frame::PixelDataFrame;
pub use pixel_data_native::{iter_pixels_color, iter_pixels_grayscale};
pub use pixel_data_reader::PixelDataReader;

use dcmfx_core::{DataSet, TransferSyntax, dictionary, transfer_syntax};
use dcmfx_p10::DataSetP10Extensions;

/// An RGB color where each component is in the range 0-1.
///
pub type RgbColor = (f64, f64, f64);

/// Adds functions to [`DataSet`] for accessing its pixel data.
///
pub trait DataSetPixelDataExtensions
where
  Self: Sized,
{
  /// Returns the frames of pixel data present in a data set.
  ///
  /// The *'(7FE0,0010) Pixel Data'* data element must be present in the data
  /// set, and the *'(0028,0008) Number of Frames'*, *'(7FE0,0001) Extended
  /// Offset Table'*, and *'(7FE0,0002) Extended Offset Table Lengths'* data
  /// elements are used when present and relevant.
  ///
  fn get_pixel_data_frames(
    &self,
  ) -> Result<Vec<PixelDataFrame>, PixelDataFilterError>;
}

impl DataSetPixelDataExtensions for DataSet {
  fn get_pixel_data_frames(
    &self,
  ) -> Result<Vec<PixelDataFrame>, PixelDataFilterError> {
    // Create a new data set containing only the data elements needed by the
    // pixel data filter. This avoids calling `DataSet::to_p10_tokens()` on the
    // whole data set.
    let mut ds = DataSet::new();
    for tag in [
      dictionary::NUMBER_OF_FRAMES.tag,
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
    let mut pixel_data_filter = PixelDataFilter::new();
    let mut frames = vec![];
    ds.to_p10_tokens(&mut |token| {
      frames.extend_from_slice(&pixel_data_filter.add_token(token)?);
      Ok(())
    })?;

    Ok(frames)
  }
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

    // JPEG 2000 uses the .jp2 extension
    ts if ts == &JPEG_2K_LOSSLESS_ONLY
      || ts == &JPEG_2K
      || ts == &JPEG_2K_MULTI_COMPONENT_LOSSLESS_ONLY
      || ts == &JPEG_2K_MULTI_COMPONENT =>
    {
      ".jp2"
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

    // High-Throughput JPEG 2000 uses the .jph extension
    ts if ts == &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2K =>
    {
      ".jph"
    }

    // Everything else uses the .bin extension as there isn't a more meaningful
    // image extension to use
    _ => ".bin",
  }
}

#[cfg(test)]
mod tests {
  use std::rc::Rc;

  use super::*;
  use dcmfx_core::{DataElementValue, ValueRepresentation, dictionary};

  #[test]
  fn read_native_empty_frame() {
    let mut ds = DataSet::new();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        Rc::new(vec![]),
      )
      .unwrap(),
    );

    assert_eq!(ds.get_pixel_data_frames().unwrap(), vec![],);
  }

  #[test]
  fn read_native_single_frame() {
    let mut ds = DataSet::new();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        Rc::new(vec![1, 2, 3, 4]),
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
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        Rc::new(vec![1, 2, 3, 4]),
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
  fn read_native_multi_frame_malformed() {
    let mut ds = DataSet::new();
    ds.insert_int_value(&dictionary::NUMBER_OF_FRAMES, &[3])
      .unwrap();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherByteString,
        Rc::new(vec![1, 2, 3, 4]),
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
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          Rc::new(vec![0, 0, 0, 0, 0x46, 0x06, 0, 0]),
          Rc::new("1".repeat(0x2C8).as_bytes().to_vec()),
          Rc::new("2".repeat(0x36E).as_bytes().to_vec()),
          Rc::new("3".repeat(0xBC8).as_bytes().to_vec()),
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
        Rc::new(vec![
          0, 0, 0, 0, 0, 0, 0, 0, 206, 4, 0, 0, 0, 0, 0, 0, 32, 7, 0, 0, 0, 0,
          0, 0,
        ]),
      )
      .unwrap(),
    );
    ds.insert(
      dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
      DataElementValue::new_binary(
        ValueRepresentation::OtherVeryLongString,
        Rc::new(vec![
          198, 4, 0, 0, 0, 0, 0, 0, 74, 2, 0, 0, 0, 0, 0, 0, 39, 6, 0, 0, 0, 0,
          0, 0,
        ]),
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
    let mut frame = PixelDataFrame::new(0);

    for fragment in fragments.iter() {
      frame.push_fragment(Rc::new(fragment.to_vec()), 0..fragment.len());
    }

    frame
  }

  fn data_set_with_three_fragments() -> DataSet {
    let mut ds = DataSet::new();
    ds.insert(
      dictionary::PIXEL_DATA.tag,
      DataElementValue::new_encapsulated_pixel_data(
        ValueRepresentation::OtherByteString,
        vec![
          Rc::new(vec![]),
          Rc::new("1".repeat(0x4C6).as_bytes().to_vec()),
          Rc::new("2".repeat(0x24A).as_bytes().to_vec()),
          Rc::new("3".repeat(0x628).as_bytes().to_vec()),
        ],
      )
      .unwrap(),
    );
    ds
  }
}
