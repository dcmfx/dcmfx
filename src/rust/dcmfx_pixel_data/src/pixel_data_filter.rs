//! Extracts frames of raw pixel data from a stream of DICOM P10 parts.

use byteorder::ByteOrder;
use std::{collections::VecDeque, rc::Rc};

use dcmfx_core::{dictionary, DataError, DataSet, ValueRepresentation};
use dcmfx_p10::{P10FilterTransform, P10Part};

use crate::PixelDataRawFrame;

/// This filter takes a stream of DICOM P10 parts and emits the frames of raw
/// pixel data it contains. Each frame is returned without any copying of pixel
/// data being performed, allowing for memory-efficient stream processing.
///
/// All native and encapsulated pixel data is supported, with the exception of
/// multi-frame native pixel data where each frame is not a whole number of
/// bytes.
///
pub struct PixelDataFilter {
  is_encapsulated: bool,

  // Filter used to extract the value of data elements needed
  details_filter: Option<P10FilterTransform>,
  details: DataSet,

  // Filter used to extract only the '(7FE0,0010) Pixel Data' data element
  pixel_data_filter: P10FilterTransform,

  // When reading native pixel data, the size of a single frame in bytes
  native_pixel_data_frame_size: usize,

  // Chunks of raw pixel data that have not yet been emitted as part of a raw
  // frame. The second value is an offset into the Vec<u8> where the un-emitted
  // raw frame data begins, which is only used for native pixel data and not for
  // encapsulated pixel data.
  pixel_data: VecDeque<(Rc<Vec<u8>>, usize)>,

  pixel_data_write_offset: u64,
  pixel_data_read_offset: u64,

  // The offset table used with encapsulated pixel data. This can come from
  // either the Basic Offset Table stored in the first pixel data item, or from
  // an Extended Offset Table.
  offset_table: Option<OffsetTable>,
}

type OffsetTable = VecDeque<(u64, Option<u64>)>;

impl PixelDataFilter {
  /// Creates a new P10 pixel data filter to extract frames of pixel data from a
  /// stream of DICOM P10 parts.
  ///
  pub fn new() -> Self {
    let details_filter = P10FilterTransform::new(
      Box::new(|tag, vr, location| {
        (tag == dictionary::NUMBER_OF_FRAMES.tag
          || tag == dictionary::EXTENDED_OFFSET_TABLE.tag
          || tag == dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag)
          && vr != ValueRepresentation::Sequence
          && location.is_empty()
      }),
      true,
    );

    let pixel_data_filter = P10FilterTransform::new(
      Box::new(|tag, _, location| {
        tag == dictionary::PIXEL_DATA.tag && location.is_empty()
      }),
      false,
    );

    Self {
      is_encapsulated: false,
      details_filter: Some(details_filter),
      details: DataSet::new(),
      pixel_data_filter,
      native_pixel_data_frame_size: 0,
      pixel_data: VecDeque::new(),
      pixel_data_write_offset: 0,
      pixel_data_read_offset: 0,
      offset_table: None,
    }
  }

  /// Adds the next DICOM P10 part, returning any frames of raw pixel data that
  /// are now available.
  ///
  pub fn add_part(
    &mut self,
    part: &P10Part,
  ) -> Result<Vec<PixelDataRawFrame>, DataError> {
    // Add the part into the details filter if it is still active
    if let Some(details_filter) = self.details_filter.as_mut() {
      details_filter.add_part(part);
    }

    if !part.is_header_part() && self.pixel_data_filter.add_part(part) {
      // If the result of the details filter hasn't yet been extracted into a
      // data set then do so now
      if let Some(details_filter) = self.details_filter.as_mut() {
        self.details = details_filter.data_set().unwrap_or(DataSet::new());
        self.details_filter = None;
      }

      self.process_next_pixel_data_part(part)
    } else {
      Ok(vec![])
    }
  }

  fn process_next_pixel_data_part(
    &mut self,
    part: &P10Part,
  ) -> Result<Vec<PixelDataRawFrame>, DataError> {
    match part {
      // The start of native pixel data
      P10Part::DataElementHeader { length, .. } => {
        self.is_encapsulated = false;

        // Check that the pixel data length divides evenly into the number of
        // frames
        let number_of_frames = self.get_number_of_frames()?;

        if *length as usize % number_of_frames != 0 {
          return Err(DataError::new_value_invalid(format!(
            "Multi-frame pixel data of length {} bytes does not divide evenly \
            into {} frames",
            *length, number_of_frames
          )));
        }

        // Store the size of native pixel data frames
        self.native_pixel_data_frame_size =
          (*length as usize) / number_of_frames;

        Ok(vec![])
      }

      // The start of encapsulated pixel data
      P10Part::SequenceStart { .. } => {
        self.is_encapsulated = true;
        Ok(vec![])
      }

      // The end of the encapsulated pixel data
      P10Part::SequenceDelimiter => {
        let mut frames = vec![];

        // If there is any remaining pixel data then emit it as a final frame
        if !self.pixel_data.is_empty() {
          let mut frame = PixelDataRawFrame::new();
          for item in self.pixel_data.iter() {
            frame.push_fragment(item.0.clone(), 0..item.0.len());
          }

          // If this frame has a length specified then apply it
          if let Some(offset_table) = self.offset_table.as_ref() {
            if let Some((_, Some(frame_length))) = offset_table.front() {
              Self::apply_length_to_frame(&mut frame, *frame_length)?;
            }
          }

          frames.push(frame);
        }

        Ok(frames)
      }

      // The start of a new encapsulated pixel data item. The size of an item
      // header is 8 bytes, and this needs to be included in the current offset.
      P10Part::PixelDataItem { .. } => {
        self.pixel_data_write_offset += 8;
        Ok(vec![])
      }

      P10Part::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } => {
        self.pixel_data.push_back((data.clone(), 0));
        self.pixel_data_write_offset += data.len() as u64;

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

  /// Returns the value for '(0028,0008) Number of Frames' data element.
  ///
  fn get_number_of_frames(&self) -> Result<usize, DataError> {
    if !self.details.has(dictionary::NUMBER_OF_FRAMES.tag) {
      return Ok(1);
    }

    let number_of_frames =
      self.details.get_int(dictionary::NUMBER_OF_FRAMES.tag)?;

    let number_of_frames = TryInto::<usize>::try_into(number_of_frames)
      .map_err(|_| {
        DataError::new_value_invalid(format!(
          "Invalid number of frames value: {}",
          number_of_frames
        ))
      })?;

    Ok(number_of_frames)
  }

  /// Consumes the native pixel data for as many frames as possible and returns
  /// them.
  ///
  fn get_pending_native_frames(
    &mut self,
  ) -> Result<Vec<PixelDataRawFrame>, DataError> {
    let mut frames = vec![];

    let frame_size = self.native_pixel_data_frame_size;

    while self.pixel_data_read_offset + frame_size as u64
      <= self.pixel_data_write_offset
    {
      let mut frame = PixelDataRawFrame::new();

      while frame.len() < frame_size {
        let (chunk, chunk_offset) = self.pixel_data.pop_front().unwrap();

        // If the whole of this chunk is needed for the next frame then add it
        // to the frame
        if chunk.len() - chunk_offset <= frame_size - frame.len() {
          frame.push_fragment(chunk.clone(), chunk_offset..chunk.len());
          self.pixel_data_read_offset += (chunk.len() - chunk_offset) as u64;
        }
        // Otherwise, take just the part of this chunk of pixel data needed
        // for the frame
        else {
          let length = frame_size - frame.len();
          frame.push_fragment(
            chunk.clone(),
            chunk_offset..(chunk_offset + length),
          );

          // Put the unused part of the chunk back on so it can be used by the
          // next frame
          self
            .pixel_data
            .push_front((chunk.clone(), chunk_offset + length));
          self.pixel_data_read_offset += length as u64;
        }
      }

      frames.push(frame);
    }

    Ok(frames)
  }

  /// Consumes the raw encapsulated pixel data for as many frames as possible
  /// and returns them.
  ///
  fn get_pending_encapsulated_frames(
    &mut self,
  ) -> Result<Vec<PixelDataRawFrame>, DataError> {
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
          // If the offset table is empty and there is more thn one frame
          // expected then each pixel data item is treated as a single frame
          // TODO: validate that this emits the number of frames is observed?
          if self.get_number_of_frames()? > 1 {
            let mut frame = PixelDataRawFrame::new();
            for (chunk, _) in self.pixel_data.iter() {
              frame.push_fragment(chunk.clone(), 0..chunk.len());
            }

            frames.push(frame);

            self.pixel_data.clear();
            self.pixel_data_read_offset = self.pixel_data_write_offset;
          }
        } else {
          // Use the offset table to determine what frames to emit
          while let Some((offset, _)) = offset_table.get(1).cloned() {
            if self.pixel_data_write_offset < offset {
              break;
            }
            let mut frame = PixelDataRawFrame::new();

            while self.pixel_data_read_offset < offset {
              if let Some((chunk, _)) = self.pixel_data.pop_front() {
                let chunk_len = chunk.len();
                frame.push_fragment(chunk, 0..chunk_len);
                self.pixel_data_read_offset += 8 + chunk_len as u64;
              } else {
                break;
              }
            }

            let (_, frame_length) = offset_table.pop_front().unwrap();

            // Check that the frame ended exactly on the expected offset
            if self.pixel_data_read_offset != offset {
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
          "Extended Offset Table must be absent when there is a Basic Offset Table"
            .to_string(),
        ));
      }

      Ok(basic_offset_table)
    }
  }

  fn read_basic_offset_table(&self) -> Result<OffsetTable, DataError> {
    // Read raw Basic Offset Table data into a buffer
    let mut offset_table_data = vec![];
    for item in self.pixel_data.iter() {
      offset_table_data.extend_from_slice(&item.0);
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

    if offsets.is_empty() {
      return Ok(VecDeque::new());
    }

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
      offset_table.push_back((offset as u64, None));
    }

    Ok(offset_table)
  }

  fn read_extended_offset_table(
    &self,
  ) -> Result<Option<OffsetTable>, DataError> {
    if !self.details.has(dictionary::EXTENDED_OFFSET_TABLE.tag) {
      return Ok(None);
    }

    // Get the value of the '(0x7FE0,0001) Extended Offset Table' data
    // element
    let extended_offset_table_bytes = self.details.get_value_bytes(
      dictionary::EXTENDED_OFFSET_TABLE.tag,
      ValueRepresentation::OtherVeryLongString,
    )?;

    if extended_offset_table_bytes.len() % 8 != 0 {
      return Err(DataError::new_value_invalid(
        "Extended Offset Table has invalid size".to_string(),
      ));
    }

    let mut extended_offset_table =
      vec![0u64; extended_offset_table_bytes.len() / 8];
    byteorder::LittleEndian::read_u64_into(
      extended_offset_table_bytes.as_slice(),
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

    // Get the value of the '(0x7FE0,0002) Extended Offset Table Lengths' data
    // element
    let extended_offset_table_lengths_bytes = self.details.get_value_bytes(
      dictionary::EXTENDED_OFFSET_TABLE_LENGTHS.tag,
      ValueRepresentation::OtherVeryLongString,
    )?;

    if extended_offset_table_lengths_bytes.len() % 8 != 0 {
      return Err(DataError::new_value_invalid(
        "Extended Offset Table Lengths has invalid size".to_string(),
      ));
    }

    let mut extended_offset_table_lengths =
      vec![0u64; extended_offset_table_lengths_bytes.len() / 8];
    byteorder::LittleEndian::read_u64_into(
      extended_offset_table_lengths_bytes.as_slice(),
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

  fn apply_length_to_frame(
    frame: &mut PixelDataRawFrame,
    frame_length: u64,
  ) -> Result<(), DataError> {
    match frame.len() as u64 {
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

impl Default for PixelDataFilter {
  fn default() -> Self {
    Self::new()
  }
}