//! Access pixel data in a DICOM data set.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use image::RgbImage;

mod color_image;
mod decode;
mod encode;
mod luts;
mod overlays;
mod p10_pixel_data_frame_filter;
mod pixel_data_definition;
mod pixel_data_frame;
mod pixel_data_renderer;
mod single_channel_image;

pub use color_image::ColorImage;
pub use luts::{
  ColorPalette, LookupTable, ModalityLut, StandardColorPalette, VoiLut,
  VoiLutFunction, VoiWindow,
};
pub use overlays::{Overlay, OverlaySubtype, OverlayType, Overlays};
pub use p10_pixel_data_frame_filter::{
  P10PixelDataFrameFilter, P10PixelDataFrameFilterError,
};
pub use pixel_data_definition::{
  BitsAllocated, PhotometricInterpretation, PixelDataDefinition,
  PixelRepresentation, PlanarConfiguration, SamplesPerPixel,
};
pub use pixel_data_frame::PixelDataFrame;
pub use pixel_data_renderer::PixelDataRenderer;
pub use single_channel_image::SingleChannelImage;

use dcmfx_core::{
  DataError, DataSet, TransferSyntax, dictionary, transfer_syntax,
};
use dcmfx_p10::DataSetP10Extensions;

/// Adds functions to [`DataSet`] for accessing its pixel data.
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
  ) -> Result<Vec<PixelDataFrame>, P10PixelDataFrameFilterError>;

  /// Returns the frames of pixel data in this data set as fully resolved RGB8
  /// images. Output image data may have been subjected to a lossy conversion to
  /// 8-bit depth.
  ///
  /// For grayscale frames, any Modality LUT and VOI LUT present in the data set
  /// are applied to reach a final grayscale value, which is duplicated across
  /// the RGB components.
  ///
  /// Grayscale values can optionally be visualized using a color palette. The
  /// well-known color palettes defined in PS3.6 B.1 are provided in
  /// [`crate::luts::color_palettes`].
  ///
  fn get_pixel_data_images(
    &self,
    color_palette: Option<&ColorPalette>,
  ) -> Result<Vec<RgbImage>, P10PixelDataFrameFilterError>;

  /// Returns the frames of pixel data in this data set as
  /// [`SingleChannelImage`]s.
  ///
  /// This will only succeed when the pixel data is single channel. Returned
  /// images needs to have the Modality LUT and VOI LUT applied in order to
  /// reach final grayscale display values.
  ///
  fn get_pixel_data_single_channel_images(
    &self,
  ) -> Result<Vec<SingleChannelImage>, P10PixelDataFrameFilterError>;

  /// Returns the frames of pixel data in this data set as [`ColorImage`]s.
  ///
  /// This will only succeed when the pixel data has three channels.
  ///
  fn get_pixel_data_color_images(
    &self,
  ) -> Result<Vec<ColorImage>, P10PixelDataFrameFilterError>;
}

impl DataSetPixelDataExtensions for DataSet {
  fn get_pixel_data_frames(
    &self,
  ) -> Result<Vec<PixelDataFrame>, P10PixelDataFrameFilterError> {
    // Create a new data set containing only the data elements needed by the
    // pixel data filter. This avoids calling `DataSet::to_p10_tokens()` on the
    // whole data set.
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
    let mut pixel_data_frame_filter = P10PixelDataFrameFilter::new();
    let mut frames = vec![];
    ds.to_p10_tokens(&mut |token| {
      frames.extend_from_slice(&pixel_data_frame_filter.add_token(token)?);
      Ok(())
    })?;

    Ok(frames)
  }

  fn get_pixel_data_images(
    &self,
    color_palette: Option<&ColorPalette>,
  ) -> Result<Vec<RgbImage>, P10PixelDataFrameFilterError> {
    get_pixel_data(self, |renderer, frame| {
      renderer.render_frame(frame, color_palette)
    })
  }

  fn get_pixel_data_single_channel_images(
    &self,
  ) -> Result<Vec<SingleChannelImage>, P10PixelDataFrameFilterError> {
    get_pixel_data(self, |renderer, frame| {
      renderer.render_single_channel_frame(frame)
    })
  }

  fn get_pixel_data_color_images(
    &self,
  ) -> Result<Vec<ColorImage>, P10PixelDataFrameFilterError> {
    get_pixel_data(self, |renderer, frame| renderer.render_color_frame(frame))
  }
}

fn get_pixel_data<T, F>(
  data_set: &DataSet,
  mut process_frame: F,
) -> Result<Vec<T>, P10PixelDataFrameFilterError>
where
  F: FnMut(&PixelDataRenderer, &mut PixelDataFrame) -> Result<T, DataError>,
{
  let renderer = PixelDataRenderer::from_data_set(data_set)
    .map_err(P10PixelDataFrameFilterError::DataError)?;

  let frames = data_set.get_pixel_data_frames()?;

  frames
    .into_iter()
    .map(|mut frame| {
      process_frame(&renderer, &mut frame)
        .map_err(P10PixelDataFrameFilterError::DataError)
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

    // JPEG XL uses the .jxl extension
    ts if ts == &JPEG_XL_LOSSLESS
      || ts == &JPEG_XL_JPEG_RECOMPRESSION
      || ts == &JPEG_XL =>
    {
      ".jxl"
    }

    // High-Throughput JPEG 2000 uses the .jph extension
    ts if ts == &HIGH_THROUGHPUT_JPEG_2K_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2K_WITH_RPCL_OPTIONS_LOSSLESS_ONLY
      || ts == &HIGH_THROUGHPUT_JPEG_2K =>
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
  #[cfg(feature = "std")]
  use std::rc::Rc;

  #[cfg(not(feature = "std"))]
  use alloc::{rc::Rc, string::ToString};

  use super::*;
  use dcmfx_core::{DataElementValue, ValueRepresentation, dictionary};

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
        Rc::new(vec![]),
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
    ds.insert_int_value(&dictionary::ROWS, &[1]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[2]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
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
        Rc::new(vec![
          0b00110001, 0b00011011, 0b10100011, 0b01000101, 0b00010101,
          0b00000110,
        ]),
      )
      .unwrap(),
    );

    let frames = ds.get_pixel_data_frames().unwrap();
    assert_eq!(*frames[0].to_bytes(), vec![0b00110001, 0b00011011]);
    assert_eq!(
      *frames[1].to_bytes(),
      vec![0b11010001, 0b10100010, 0b10000000]
    );
    assert_eq!(
      *frames[2].to_bytes(),
      vec![0b01000101, 0b01000001, 0b10000000]
    );
    assert_eq!(frames[0].bit_offset(), 0);
    assert_eq!(frames[1].bit_offset(), 7);
    assert_eq!(frames[2].bit_offset(), 6);
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
    ds.insert_int_value(&dictionary::ROWS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
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
    ds.insert_int_value(&dictionary::ROWS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::COLUMNS, &[0]).unwrap();
    ds.insert_int_value(&dictionary::BITS_ALLOCATED, &[8])
      .unwrap();
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
