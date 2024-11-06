//! Defines the various parts of a DICOM P10 that are read out of raw DICOM P10
//! data by the `p10_read` module.

use std::rc::Rc;

use dcmfx_core::{
  registry, DataElementTag, DataElementValue, DataSet, ValueRepresentation,
};

use crate::internal::data_element_header::DataElementHeader;

/// A DICOM P10 part is the smallest piece of structured DICOM P10 data, and a
/// stream of these parts is most commonly the result of progressive reading of
/// raw DICOM P10 bytes, or from conversion of a data set into P10 parts for
/// transmission or serialization.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10Part {
  /// The 128-byte File Preamble and the "DICM" prefix, which are present at the
  /// start of DICOM P10 data. The content of the File Preamble's bytes are
  /// application-defined, and in many cases are unused and set to zero.
  ///
  /// When reading DICOM P10 data that doesn't contain a File Preamble and
  /// "DICM" prefix this part is emitted with all bytes set to zero.
  FilePreambleAndDICMPrefix { preamble: Box<[u8; 128]> },

  /// The File Meta Information dataset for the DICOM P10.
  ///
  /// When reading DICOM P10 data that doesn't contain File Meta Information
  /// this part is emitted with an empty data set.
  FileMetaInformation { data_set: DataSet },

  /// The start of the next data element. This part will always be followed by
  /// one or more [`P10Part::DataElementValueBytes`] parts containing the value
  /// bytes for the data element.
  DataElementHeader {
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: u32,
  },

  /// Raw data for the value of the current data element. Data element values
  /// are split across multiple of these parts when their length exceeds the
  /// maximum part size.
  DataElementValueBytes {
    vr: ValueRepresentation,
    data: Rc<Vec<u8>>,
    bytes_remaining: u32,
  },

  /// The start of a new sequence. If this is the start of a sequence of
  /// encapsulated pixel data then the VR of that data, either
  /// [`ValueRepresentation::OtherByteString`] or
  /// [`ValueRepresentation::OtherWordString`], will be specified. If not, the
  /// VR will be [`ValueRepresentation::Sequence`].
  SequenceStart {
    tag: DataElementTag,
    vr: ValueRepresentation,
  },

  /// The end of the current sequence.
  SequenceDelimiter,

  /// The start of a new item in the current sequence.
  SequenceItemStart,

  /// The end of the current sequence item.
  SequenceItemDelimiter,

  /// The start of a new item in the current encapsulated pixel data sequence.
  /// The data for the item follows in one or more
  /// [`P10Part::DataElementValueBytes`] parts.
  PixelDataItem { length: u32 },

  /// The end of the DICOM P10 data has been reached with all provided data
  /// successfully parsed.
  End,
}

impl std::fmt::Display for P10Part {
  /// Converts a DICOM P10 part to a human-readable string.
  ///
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let s = match self {
      P10Part::FilePreambleAndDICMPrefix { .. } => {
        "FilePreambleAndDICMPrefix".to_string()
      }

      P10Part::FileMetaInformation { data_set } => {
        format!(
          "FileMetaInformation: {}",
          data_set
            .iter()
            .map(|(tag, value)| {
              format!(
                "{}: {}",
                DataElementHeader {
                  tag: *tag,
                  vr: Some(value.value_representation()),
                  length: 0,
                },
                value.to_string(*tag, 80)
              )
            })
            .collect::<Vec<String>>()
            .join(", ")
        )
      }

      P10Part::DataElementHeader { tag, vr, length } => format!(
        "DataElementHeader: {}, name: {}, vr: {}, length: {} bytes",
        tag,
        registry::tag_name(*tag, None),
        vr,
        length
      ),

      P10Part::DataElementValueBytes {
        vr: _vr,
        data,
        bytes_remaining,
      } => format!(
        "DataElementValueBytes: {} bytes of data, {} bytes remaining",
        data.len(),
        bytes_remaining
      ),

      P10Part::SequenceStart { tag, vr } => format!(
        "SequenceStart: {}, name: {}, vr: {}",
        tag,
        registry::tag_name(*tag, None),
        vr,
      ),

      P10Part::SequenceDelimiter => "SequenceDelimiter".to_string(),

      P10Part::SequenceItemStart => "SequenceItemStart".to_string(),

      P10Part::SequenceItemDelimiter => "SequenceItemDelimiter".to_string(),

      P10Part::PixelDataItem { length } => {
        format!("PixelDataItem: {} bytes", length)
      }

      P10Part::End => "End".to_string(),
    };

    write!(f, "{}", s)
  }
}

/// Converts all the data elements in a data set directly to DICOM P10 parts.
/// Each part is returned via a callback.
///
pub fn data_elements_to_parts<E>(
  data_set: &DataSet,
  part_callback: &mut impl FnMut(&P10Part) -> Result<(), E>,
) -> Result<(), E> {
  for (tag, value) in data_set.iter() {
    data_element_to_parts(*tag, value, part_callback)?;
  }

  Ok(())
}

/// Converts a DICOM data element to DICOM P10 parts. Each part is returned via
/// a callback.
///
pub fn data_element_to_parts<E>(
  tag: DataElementTag,
  value: &DataElementValue,
  part_callback: &mut impl FnMut(&P10Part) -> Result<(), E>,
) -> Result<(), E> {
  let vr = value.value_representation();

  let length = match value.bytes() {
    Ok(bytes) => bytes.len(),
    Err(_) => 0xFFFFFFFF,
  } as u32;

  // For values that have their bytes directly available write them out as-is
  if let Ok(bytes) = value.bytes() {
    let header_part = P10Part::DataElementHeader { tag, vr, length };
    part_callback(&header_part)?;

    part_callback(&P10Part::DataElementValueBytes {
      vr,
      data: bytes.clone(),
      bytes_remaining: 0,
    })?;

    return Ok(());
  }

  // For encapsulated pixel data, write all of the items individually,
  // followed by a sequence delimiter
  if let Ok(items) = value.encapsulated_pixel_data() {
    let header_part = P10Part::SequenceStart { tag, vr };
    part_callback(&header_part)?;

    for item in items {
      let length = item.len() as u32;
      let item_header_part = P10Part::PixelDataItem { length };

      part_callback(&item_header_part)?;

      let value_bytes_part = P10Part::DataElementValueBytes {
        vr,
        data: item.clone(),
        bytes_remaining: 0,
      };
      part_callback(&value_bytes_part)?;
    }

    // Write delimiter for the encapsulated pixel data sequence
    part_callback(&P10Part::SequenceDelimiter)?;

    return Ok(());
  }

  // For sequences, write the item data sets recursively, followed by a
  // sequence delimiter
  if let Ok(items) = value.sequence_items() {
    let header_part = P10Part::SequenceStart { tag, vr };
    part_callback(&header_part)?;

    for item in items {
      let item_start_part = P10Part::SequenceItemStart;
      part_callback(&item_start_part)?;

      data_elements_to_parts(item, part_callback)?;

      // Write delimiter for the item
      let item_delimiter_part = P10Part::SequenceItemDelimiter;
      part_callback(&item_delimiter_part)?;
    }

    // Write delimiter for the sequence
    part_callback(&P10Part::SequenceDelimiter)?;

    return Ok(());
  }

  // It isn't logically possible to reach here as one of the above branches must
  // have been taken
  unreachable!();
}
