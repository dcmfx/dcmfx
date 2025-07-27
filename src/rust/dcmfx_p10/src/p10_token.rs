//! Defines the various tokens of a DICOM P10 that are read out of raw DICOM P10
//! data by the `p10_read` module.

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec::Vec,
};

use dcmfx_core::{
  DataElementTag, DataElementValue, DataSet, DataSetPath, RcByteSlice,
  TransferSyntax, ValueRepresentation, dictionary,
};

use crate::internal::{
  data_element_header::DataElementHeader, value_length::ValueLength,
};

/// A DICOM P10 token is the smallest piece of structured DICOM P10 data, and a
/// stream of these tokens is most commonly the result of progressive reading of
/// raw DICOM P10 bytes, or from conversion of a data set into P10 tokens for
/// transmission or serialization.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10Token {
  /// The 128-byte File Preamble and the "DICM" prefix, which are present at the
  /// start of DICOM P10 data. The content of the File Preamble's bytes are
  /// application-defined, and in many cases are unused and set to zero.
  ///
  /// When reading DICOM P10 data that doesn't contain a File Preamble and
  /// "DICM" prefix this token is emitted with all bytes set to zero.
  FilePreambleAndDICMPrefix { preamble: Box<[u8; 128]> },

  /// The File Meta Information dataset for the DICOM P10.
  ///
  /// When reading DICOM P10 data that doesn't contain File Meta Information
  /// this token is emitted with an empty data set.
  FileMetaInformation { data_set: DataSet },

  /// The start of the next data element. This token will always be followed by
  /// one or more [`P10Token::DataElementValueBytes`] tokens containing the
  /// value bytes for the data element.
  DataElementHeader {
    tag: DataElementTag,
    vr: ValueRepresentation,
    length: u32,
    path: DataSetPath,
  },

  /// Raw data for the value of the current data element. Data element values
  /// are split across multiple of these tokens when their length exceeds the
  /// maximum token size.
  DataElementValueBytes {
    tag: DataElementTag,
    vr: ValueRepresentation,
    data: RcByteSlice,
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
    path: DataSetPath,
  },

  /// The end of the current sequence.
  SequenceDelimiter { tag: DataElementTag },

  /// The start of a new item in the current sequence.
  SequenceItemStart { index: usize },

  /// The end of the current sequence item.
  SequenceItemDelimiter,

  /// The start of a new item in the current encapsulated pixel data sequence.
  /// The data for the item follows in one or more
  /// [`P10Token::DataElementValueBytes`] tokens.
  PixelDataItem { index: usize, length: u32 },

  /// The end of the DICOM P10 data has been reached with all provided data
  /// successfully parsed.
  End,
}

impl core::fmt::Display for P10Token {
  /// Converts a DICOM P10 token to a human-readable string.
  ///
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    let s = match self {
      P10Token::FilePreambleAndDICMPrefix { .. } => {
        "FilePreambleAndDICMPrefix".to_string()
      }

      P10Token::FileMetaInformation { data_set } => {
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
                  length: ValueLength::ZERO,
                },
                value.to_string(*tag, 80)
              )
            })
            .collect::<Vec<String>>()
            .join(", ")
        )
      }

      P10Token::DataElementHeader {
        tag, vr, length, ..
      } => format!(
        "DataElementHeader: {}, name: {}, vr: {}, length: {} bytes",
        tag,
        dictionary::tag_name(*tag, None),
        vr,
        length
      ),

      P10Token::DataElementValueBytes {
        data,
        bytes_remaining,
        ..
      } => format!(
        "DataElementValueBytes: {} bytes of data, {} bytes remaining",
        data.len(),
        bytes_remaining
      ),

      P10Token::SequenceStart { tag, vr, .. } => format!(
        "SequenceStart: {}, name: {}, vr: {}",
        tag,
        dictionary::tag_name(*tag, None),
        vr,
      ),

      P10Token::SequenceDelimiter { .. } => "SequenceDelimiter".to_string(),

      P10Token::SequenceItemStart { index } => {
        format!("SequenceItemStart: item {index}")
      }

      P10Token::SequenceItemDelimiter => "SequenceItemDelimiter".to_string(),

      P10Token::PixelDataItem { index, length } => {
        format!("PixelDataItem: item {index}, {length} bytes")
      }

      P10Token::End => "End".to_string(),
    };

    write!(f, "{s}")
  }
}

impl P10Token {
  /// Returns whether this DICOM P10 token is part of the file header or File
  /// Meta Information prior to the main data set, i.e. is it a
  /// [`P10Token::FilePreambleAndDICMPrefix`] or [`P10Token::FileMetaInformation`]
  /// token.
  ///
  pub fn is_header_token(&self) -> bool {
    matches!(
      self,
      P10Token::FilePreambleAndDICMPrefix { .. }
        | P10Token::FileMetaInformation { .. }
    )
  }

  /// If this token is a [`P10Token::FileMetaInformation`] then changes the
  /// transfer syntax it specifies. Other token types are unchanged.
  ///
  pub fn change_transfer_syntax(
    &mut self,
    new_transfer_syntax: &TransferSyntax,
  ) {
    if let P10Token::FileMetaInformation { data_set } = self {
      data_set
        .insert_string_value(
          &dictionary::TRANSFER_SYNTAX_UID,
          &[new_transfer_syntax.uid],
        )
        .unwrap();
    }
  }
}

/// Converts all the data elements in a data set directly to DICOM P10 tokens.
/// Each token is returned via a callback.
///
pub fn data_elements_to_tokens<E>(
  data_set: &DataSet,
  path: &DataSetPath,
  token_callback: &mut impl FnMut(&P10Token) -> Result<(), E>,
) -> Result<(), E> {
  for (tag, value) in data_set.iter() {
    let mut path = path.clone();
    path.add_data_element(*tag).unwrap();

    data_element_to_tokens(*tag, value, &path, token_callback)?;
  }

  Ok(())
}

/// Converts a DICOM data element to DICOM P10 tokens. Each token is returned
/// via a callback.
///
pub fn data_element_to_tokens<E>(
  tag: DataElementTag,
  value: &DataElementValue,
  path: &DataSetPath,
  token_callback: &mut impl FnMut(&P10Token) -> Result<(), E>,
) -> Result<(), E> {
  let vr = value.value_representation();

  // For values that have their bytes directly available write them out as-is
  if let Ok(bytes) = value.bytes() {
    let header_token = P10Token::DataElementHeader {
      tag,
      vr,
      length: bytes.len() as u32,
      path: path.clone(),
    };
    token_callback(&header_token)?;

    token_callback(&P10Token::DataElementValueBytes {
      tag,
      vr,
      data: bytes.clone(),
      bytes_remaining: 0,
    })?;

    return Ok(());
  }

  // For encapsulated pixel data, write all of the items individually,
  // followed by a sequence delimiter
  if let Ok(items) = value.encapsulated_pixel_data() {
    let header_token = P10Token::SequenceStart {
      tag,
      vr,
      path: path.clone(),
    };
    token_callback(&header_token)?;

    for (index, item) in items.iter().enumerate() {
      let length = item.len() as u32;
      let item_header_token = P10Token::PixelDataItem { index, length };

      token_callback(&item_header_token)?;

      let value_bytes_token = P10Token::DataElementValueBytes {
        tag: dictionary::ITEM.tag,
        vr,
        data: item.clone(),
        bytes_remaining: 0,
      };
      token_callback(&value_bytes_token)?;
    }

    // Write delimiter for the encapsulated pixel data sequence
    token_callback(&P10Token::SequenceDelimiter { tag })?;

    return Ok(());
  }

  // For sequences, write the item data sets recursively, followed by a
  // sequence delimiter
  if let Ok(items) = value.sequence_items() {
    let header_token = P10Token::SequenceStart {
      tag,
      vr,
      path: path.clone(),
    };
    token_callback(&header_token)?;

    for (index, item) in items.iter().enumerate() {
      let item_start_token = P10Token::SequenceItemStart { index };
      token_callback(&item_start_token)?;

      let mut path = path.clone();
      path.add_sequence_item(index).unwrap();

      data_elements_to_tokens(item, &path, token_callback)?;

      // Write delimiter for the item
      let item_delimiter_token = P10Token::SequenceItemDelimiter;
      token_callback(&item_delimiter_token)?;
    }

    // Write delimiter for the sequence
    token_callback(&P10Token::SequenceDelimiter { tag })?;

    return Ok(());
  }

  // It isn't logically possible to reach here as one of the above branches must
  // have been taken
  unreachable!();
}
