//! Extracts frames of pixel data from a stream of DICOM P10 tokens.

#[cfg(feature = "std")]
use std::collections::VecDeque;

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  collections::VecDeque,
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use byteorder::ByteOrder;

use dcmfx_core::{
  DataElementValue, DataError, DataSet, DcmfxError, RcByteSlice,
  ValueRepresentation, dictionary,
};
use dcmfx_p10::{
  P10CustomTypeTransform, P10CustomTypeTransformError, P10Error,
  P10FilterTransform, P10Token,
};

use crate::PixelDataFrame;

/// This transform takes a stream of DICOM P10 tokens and emits the frames of
/// pixel data it contains. Each frame is returned with no copying of pixel
/// data, allowing for memory-efficient stream processing.
///
/// All native and encapsulated pixel data is supported.
///
pub struct P10PixelDataFrameTransform {
  is_encapsulated: bool,

  // Extracts the value of relevant data elements from the stream of DICOM P10
  // tokens
  details: P10CustomTypeTransform<PixelDataFrameTransformDetails>,

  // Filter used to extract only the '(7FE0,0010) Pixel Data' data element
  pixel_data_filter: P10FilterTransform,

  // When reading native pixel data, the size of a single frame in bits
  native_pixel_data_frame_size: u64,

  // Chunks of pixel data that have not yet been emitted as part of a frame. The
  // second value is an offset into the Vec<u8> where the un-emitted frame data
  // begins, which is only used for native pixel data and not for encapsulated
  // pixel data.
  pixel_data: VecDeque<(RcByteSlice, u64)>,

  pixel_data_write_offset: u64,
  pixel_data_read_offset: u64,

  // The offset table used with encapsulated pixel data. This can come from
  // either the Basic Offset Table stored in the first pixel data item, or from
  // an Extended Offset Table.
  offset_table: Option<OffsetTable>,

  next_frame_index: usize,
}

type OffsetTable = VecDeque<(u64, Option<u64>)>;

#[derive(Clone, Debug, PartialEq)]
struct PixelDataFrameTransformDetails {
  number_of_frames: usize,
  rows: u16,
  columns: u16,
  bits_allocated: u16,
  extended_offset_table: Option<DataElementValue>,
  extended_offset_table_lengths: Option<DataElementValue>,
}

impl PixelDataFrameTransformDetails {
  fn from_data_set(data_set: &DataSet) -> Result<Self, DataError> {
    let number_of_frames = data_set
      .get_int_with_default::<usize>(dictionary::NUMBER_OF_FRAMES.tag, 1)?;

    Ok(Self {
      number_of_frames,
      rows: data_set.get_int::<u16>(dictionary::ROWS.tag)?,
      columns: data_set.get_int::<u16>(dictionary::COLUMNS.tag)?,
      bits_allocated: data_set
        .get_int::<u16>(dictionary::BITS_ALLOCATED.tag)?,
      extended_offset_table: data_set
        .get_value(dictionary::EXTENDED_OFFSET_TABLE.tag)
        .ok()
        .cloned(),
      extended_offset_table_lengths: data_set
        .get_value(dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag)
        .ok()
        .cloned(),
    })
  }
}

/// An error that occurred in the process of extracting frames of pixel data
/// from a stream of DICOM P10 tokens.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10PixelDataFrameTransformError {
  /// An error that occurred when adding a P10 token. This can happen when the
  /// stream of DICOM P10 tokens is invalid.
  P10Error(P10Error),

  /// An error that occurred when reading the data from the data elements in the
  /// stream of DICOM P10 tokens.
  DataError(DataError),
}

impl core::fmt::Display for P10PixelDataFrameTransformError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::DataError(e) => e.fmt(f),
      Self::P10Error(e) => e.fmt(f),
    }
  }
}

impl DcmfxError for P10PixelDataFrameTransformError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::P10Error(e) => e.to_lines(task_description),
      Self::DataError(e) => e.to_lines(task_description),
    }
  }
}

impl P10PixelDataFrameTransform {
  /// Creates a new P10 pixel data frame transform to extract frames of pixel
  /// data from a stream of DICOM P10 tokens.
  ///
  pub fn new() -> Self {
    let details_transform =
      P10CustomTypeTransform::<PixelDataFrameTransformDetails>::new(
        &[
          dictionary::NUMBER_OF_FRAMES.tag,
          dictionary::ROWS.tag,
          dictionary::COLUMNS.tag,
          dictionary::BITS_ALLOCATED.tag,
          dictionary::EXTENDED_OFFSET_TABLE.tag,
          dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
        ],
        PixelDataFrameTransformDetails::from_data_set,
      );

    let pixel_data_filter =
      P10FilterTransform::new(Box::new(|tag, _vr, _length, path| {
        tag == dictionary::PIXEL_DATA.tag && path.is_root()
      }));

    Self {
      is_encapsulated: false,
      details: details_transform,
      pixel_data_filter,
      native_pixel_data_frame_size: 0,
      pixel_data: VecDeque::new(),
      pixel_data_write_offset: 0,
      pixel_data_read_offset: 0,
      offset_table: None,
      next_frame_index: 0,
    }
  }

  /// Adds the next DICOM P10 token, returning any frames of pixel data that are
  /// now available.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<PixelDataFrame>, P10PixelDataFrameTransformError> {
    // Add the token into the details transform
    match self.details.add_token(token) {
      Ok(()) => (),
      Err(P10CustomTypeTransformError::P10Error(e)) => {
        return Err(P10PixelDataFrameTransformError::P10Error(e));
      }
      Err(P10CustomTypeTransformError::DataError(e)) => {
        return Err(P10PixelDataFrameTransformError::DataError(e));
      }
    };

    if !token.is_header_token()
      && self
        .pixel_data_filter
        .add_token(token)
        .map_err(P10PixelDataFrameTransformError::P10Error)?
    {
      self
        .process_next_pixel_data_token(token)
        .map_err(P10PixelDataFrameTransformError::DataError)
    } else {
      Ok(vec![])
    }
  }

  fn process_next_pixel_data_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<PixelDataFrame>, DataError> {
    match token {
      // The start of native pixel data
      P10Token::DataElementHeader { length, .. } => {
        self.is_encapsulated = false;

        // Check that the pixel data length divides evenly into the number of
        // frames
        let number_of_frames = self.get_number_of_frames();

        if number_of_frames > 0 {
          let details = self.details.get_output().unwrap();

          // Validate the pixel data length and store the size in bits of native
          // pixel data frames
          self.native_pixel_data_frame_size = if details.bits_allocated == 1 {
            let pixel_count = details.rows as u64 * details.columns as u64;
            let expected_length =
              (pixel_count * number_of_frames as u64).div_ceil(8);

            if u64::from(*length) != expected_length {
              return Err(DataError::new_value_invalid(format!(
                "Bitmap pixel data has length {} bytes but {} bytes were \
                 expected",
                *length, expected_length
              )));
            }

            pixel_count
          } else {
            if *length as usize % number_of_frames != 0 {
              return Err(DataError::new_value_invalid(format!(
                "Multi-frame pixel data of length {} bytes does not divide \
                evenly into {} frames",
                *length, number_of_frames
              )));
            }

            (u64::from(*length) * 8) / (number_of_frames as u64)
          };
        }

        Ok(vec![])
      }

      // The start of encapsulated pixel data
      P10Token::SequenceStart { .. } => {
        self.is_encapsulated = true;
        Ok(vec![])
      }

      // The end of the encapsulated pixel data
      P10Token::SequenceDelimiter { .. } => {
        let mut frames = vec![];

        // If there is any remaining pixel data then emit it as a final frame
        if !self.pixel_data.is_empty() {
          let mut frame = PixelDataFrame::new();
          frame.set_index(self.next_frame_index);

          for item in self.pixel_data.iter() {
            frame.push_bytes(item.0.clone());
          }

          // If this frame has a length specified then apply it
          if let Some(offset_table) = self.offset_table.as_ref()
            && let Some((_, Some(frame_length))) = offset_table.front()
          {
            Self::apply_length_to_frame(&mut frame, *frame_length)?;
          }

          frames.push(frame);
        }

        Ok(frames)
      }

      // The start of a new encapsulated pixel data item. The size of an item
      // header is 8 bytes, and this needs to be included in the current offset.
      P10Token::PixelDataItem { .. } => {
        self.pixel_data_write_offset += 64;
        Ok(vec![])
      }

      P10Token::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } => {
        self.pixel_data.push_back((data.clone(), 0));
        self.pixel_data_write_offset += data.len() as u64 * 8;

        if self.is_encapsulated {
          if *bytes_remaining == 0 {
            self.get_pending_encapsulated_frames()
          } else {
            Ok(vec![])
          }
        } else if self.native_pixel_data_frame_size > 0 {
          self.get_pending_native_frames()
        } else {
          Ok(vec![])
        }
      }

      _ => Ok(vec![]),
    }
  }

  /// Returns the value for the *'(0028,0008) Number of Frames'* data element.
  ///
  pub fn get_number_of_frames(&self) -> usize {
    match self.details.get_output() {
      Some(details) => details.number_of_frames,
      None => 1,
    }
  }

  /// Consumes native pixel data for as many frames as possible and returns
  /// them.
  ///
  fn get_pending_native_frames(
    &mut self,
  ) -> Result<Vec<PixelDataFrame>, DataError> {
    let mut frames = vec![];

    let frame_size = self.native_pixel_data_frame_size;

    while self.pixel_data_read_offset + frame_size
      <= self.pixel_data_write_offset
    {
      let mut frame = PixelDataFrame::new();

      frame.set_index(self.next_frame_index);
      frame.set_bit_offset(self.pixel_data_read_offset as usize % 8);

      while frame.len_bits() < frame_size {
        let (chunk, chunk_offset) = self.pixel_data.pop_front().unwrap();

        // If the whole of this chunk is needed for the next frame then add it
        // to the frame
        if chunk.len() as u64 * 8 - chunk_offset
          <= frame_size - frame.len_bits()
        {
          frame.push_bytes(chunk.drop((chunk_offset / 8) as usize));
          self.pixel_data_read_offset += chunk.len() as u64 * 8 - chunk_offset;
        }
        // Otherwise, take just the part of this chunk of pixel data needed for
        // the frame
        else {
          let length_in_bits = frame_size - frame.len_bits();
          frame.push_bytes(chunk.slice(
            (chunk_offset / 8) as usize,
            (chunk_offset + length_in_bits).div_ceil(8) as usize,
          ));

          // Put the unused part of the chunk back on so it can be used by the
          // next frame
          self
            .pixel_data
            .push_front((chunk, (chunk_offset + length_in_bits)));
          self.pixel_data_read_offset += length_in_bits;
        }
      }

      // For native frame data, don't emit more frames than is specified by the
      // '(0028,0008) Number of Frames' data element. This is important in the
      // case of 1bpp pixel data when there are unused bits at the end of the
      // data and there are enough unused bits to contain data for one or more
      // frames. This can occur when the size of a single frame is <= 7 bits.
      if self.next_frame_index < self.get_number_of_frames() {
        frames.push(frame);
      }

      self.next_frame_index += 1;
    }

    Ok(frames)
  }

  /// Consumes encapsulated pixel data for as many frames as possible and
  /// returns them.
  ///
  fn get_pending_encapsulated_frames(
    &mut self,
  ) -> Result<Vec<PixelDataFrame>, DataError> {
    match self.offset_table.as_mut() {
      // If the Basic Offset Table hasn't been read yet, read it now that the
      // first pixel data item is complete
      None => {
        self.offset_table = Some(self.read_offset_table()?);
        self.pixel_data.clear();
        self.pixel_data_write_offset = 0;
        self.pixel_data_read_offset = 0;

        Ok(vec![])
      }

      Some(offset_table) => {
        let mut frames = vec![];

        if offset_table.is_empty() {
          // If the offset table is empty and there is more than one frame
          // then each pixel data item is treated as a single frame
          if self.get_number_of_frames() > 1 {
            let mut frame = PixelDataFrame::new();

            frame.set_index(self.next_frame_index);
            self.next_frame_index += 1;

            for (chunk, _) in self.pixel_data.iter() {
              frame.push_bytes(chunk.clone());
            }

            frames.push(frame);

            self.pixel_data.clear();
            self.pixel_data_read_offset = self.pixel_data_write_offset;
          }
        } else {
          // Use the offset table to determine what frames to emit
          while let Some((offset, _)) = offset_table.get(1).cloned() {
            if self.pixel_data_write_offset < offset * 8 {
              break;
            }

            let mut frame = PixelDataFrame::new();

            frame.set_index(self.next_frame_index);
            self.next_frame_index += 1;

            while self.pixel_data_read_offset < offset * 8 {
              if let Some((chunk, _)) = self.pixel_data.pop_front() {
                frame.push_bytes(chunk.clone());
                self.pixel_data_read_offset += (8 + chunk.len() as u64) * 8;
              } else {
                break;
              }
            }

            let (_, frame_length) = offset_table.pop_front().unwrap();

            // Check that the frame ended exactly on the expected offset
            if self.pixel_data_read_offset != offset * 8 {
              return Err(DataError::new_value_invalid(
                "Pixel data offset table is malformed".to_string(),
              ));
            }

            // If this frame has a length specified then validate and apply it
            if let Some(frame_length) = frame_length {
              Self::apply_length_to_frame(&mut frame, frame_length)?;
            }

            frames.push(frame);
          }
        }

        Ok(frames)
      }
    }
  }

  fn read_offset_table(&self) -> Result<OffsetTable, DataError> {
    let basic_offset_table = self.read_basic_offset_table()?;
    let extended_offset_table = self.read_extended_offset_table()?;

    // If the Basic Offset Table is empty then use the Extended Offset Table if
    // present. If neither are present then there is no offset table.
    if basic_offset_table.is_empty() {
      Ok(extended_offset_table.unwrap_or(VecDeque::new()))
    } else {
      // Validate that the Extended Offset Table is empty. Ref: PS3.5 A.4.
      if extended_offset_table.is_some() {
        return Err(DataError::new_value_invalid(
          "Extended Offset Table must be absent when there is a Basic Offset \
           Table"
            .to_string(),
        ));
      }

      Ok(basic_offset_table)
    }
  }

  fn read_basic_offset_table(&self) -> Result<OffsetTable, DataError> {
    // Read Basic Offset Table data into a buffer
    let mut offset_table_data = vec![];
    for item in self.pixel_data.iter() {
      offset_table_data.extend_from_slice(&item.0);
    }

    if offset_table_data.is_empty() {
      return Ok(VecDeque::new());
    }

    // Validate the data's length is a multiple of 4
    if offset_table_data.len() % 4 != 0 {
      return Err(DataError::new_value_invalid(
        "Basic Offset Table length is not a multiple of 4".to_string(),
      ));
    }

    // Read data into u32 values
    let mut offsets = vec![0u32; offset_table_data.len() / 4];
    byteorder::LittleEndian::read_u32_into(&offset_table_data, &mut offsets);

    // Check that the first offset is zero. Ref: PS3.5 A.4.
    if offsets.first() != Some(&0) {
      return Err(DataError::new_value_invalid(
        "Basic Offset Table first value must be zero".to_string(),
      ));
    }

    // Check that the offsets are sorted
    if !offsets.is_sorted() {
      return Err(DataError::new_value_invalid(
        "Basic Offset Table values are not sorted".to_string(),
      ));
    }

    let mut offset_table = VecDeque::new();
    for offset in offsets {
      offset_table.push_back((u64::from(offset), None));
    }

    Ok(offset_table)
  }

  fn read_extended_offset_table(
    &self,
  ) -> Result<Option<OffsetTable>, DataError> {
    match self.details.get_output() {
      Some(PixelDataFrameTransformDetails {
        extended_offset_table: Some(extended_offset_table),
        extended_offset_table_lengths: Some(extended_offset_table_lengths),
        ..
      }) => {
        // Get the value of the '(0x7FE0,0001) Extended Offset Table' data
        // element
        let extended_offset_table_bytes = extended_offset_table
          .vr_bytes(&[ValueRepresentation::OtherVeryLongString])?;

        if extended_offset_table_bytes.len() % 8 != 0 {
          return Err(DataError::new_value_invalid(
            "Extended Offset Table has invalid size".to_string(),
          ));
        }

        let mut extended_offset_table =
          vec![0u64; extended_offset_table_bytes.len() / 8];
        byteorder::LittleEndian::read_u64_into(
          extended_offset_table_bytes,
          extended_offset_table.as_mut_slice(),
        );

        // Check that the first offset is zero
        if *extended_offset_table.first().unwrap_or(&0) != 0 {
          return Err(DataError::new_value_invalid(
            "Extended Offset Table first value must be zero".to_string(),
          ));
        }

        // Check that the offsets are sorted
        if !extended_offset_table.is_sorted() {
          return Err(DataError::new_value_invalid(
            "Extended Offset Table values are not sorted".to_string(),
          ));
        }

        // Get the value of the '(0x7FE0,0002) Extended Offset Table Lengths'
        // data element
        let extended_offset_table_lengths_bytes = extended_offset_table_lengths
          .vr_bytes(&[ValueRepresentation::OtherVeryLongString])?;

        if extended_offset_table_lengths_bytes.len() % 8 != 0 {
          return Err(DataError::new_value_invalid(
            "Extended Offset Table Lengths has invalid size".to_string(),
          ));
        }

        let mut extended_offset_table_lengths =
          vec![0u64; extended_offset_table_lengths_bytes.len() / 8];
        byteorder::LittleEndian::read_u64_into(
          extended_offset_table_lengths_bytes,
          extended_offset_table_lengths.as_mut_slice(),
        );

        // Check the two are of the same length
        if extended_offset_table.len() != extended_offset_table_lengths.len() {
          return Err(DataError::new_value_invalid(
            "Extended Offset Table and Lengths don't have the same number of \
          items"
              .to_string(),
          ));
        }

        // Return the offset table
        let mut entries = VecDeque::with_capacity(extended_offset_table.len());
        for i in 0..extended_offset_table.len() {
          entries.push_back((
            extended_offset_table[i],
            Some(extended_offset_table_lengths[i]),
          ));
        }

        Ok(Some(entries))
      }

      _ => Ok(None),
    }
  }

  fn apply_length_to_frame(
    frame: &mut PixelDataFrame,
    frame_length: u64,
  ) -> Result<(), DataError> {
    match frame.len() {
      len if len == frame_length => (),

      len if len > frame_length => {
        frame.drop_end_bytes((len - frame_length) as usize);
      }

      _ => {
        return Err(DataError::new_value_invalid(format!(
          "Extended Offset Table Length value '{}' is invalid for \
          frame of length '{}'",
          frame_length,
          frame.len()
        )));
      }
    }

    Ok(())
  }
}

impl Default for P10PixelDataFrameTransform {
  fn default() -> Self {
    Self::new()
  }
}
