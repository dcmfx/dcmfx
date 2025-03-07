//! A data set builder materializes a stream of DICOM P10 tokens into an
//! in-memory data set.
//!
//! Most commonly the stream of DICOM P10 tokens originates from reading raw
//! DICOM P10 data with the [`crate::p10_read`] module.

#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  rc::Rc,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{
  DataElementTag, DataElementValue, DataSet, ValueRepresentation, dictionary,
};

use crate::{P10Error, P10Token};

/// A data set builder that can be fed a stream of DICOM P10 tokens and
/// materialize them into an in-memory data set.
///
#[derive(Debug, PartialEq)]
pub struct DataSetBuilder {
  file_preamble: Option<Box<[u8; 128]>>,
  file_meta_information: Option<DataSet>,
  location: Vec<BuilderLocation>,
  pending_data_element: Option<PendingDataElement>,
  is_complete: bool,
}

/// Tracks where in the data set the builder is currently at, specifically the
/// sequences and sequence items currently in the process of being created.
///
#[derive(Debug, PartialEq)]
enum BuilderLocation {
  RootDataSet {
    data_set: DataSet,
  },
  Sequence {
    tag: DataElementTag,
    items: Vec<DataSet>,
  },
  SequenceItem {
    data_set: DataSet,
  },
  EncapsulatedPixelDataSequence {
    vr: ValueRepresentation,
    items: Vec<Rc<Vec<u8>>>,
  },
}

/// The pending data element is a data element for which a `DataElementHeader`
/// token has been received, but one or more of its `DataElementValueBytes`
/// tokens are still pending.
///
#[derive(Debug, PartialEq)]
struct PendingDataElement {
  tag: DataElementTag,
  vr: ValueRepresentation,
  data: Vec<Rc<Vec<u8>>>,
}

impl Default for DataSetBuilder {
  fn default() -> Self {
    Self::new()
  }
}

impl DataSetBuilder {
  /// Creates a new data set builder that can be given DICOM P10 tokens to be
  /// materialized into an in-memory DICOM data set.
  ///
  pub fn new() -> Self {
    Self {
      file_preamble: None,
      file_meta_information: None,
      location: vec![BuilderLocation::RootDataSet {
        data_set: DataSet::new(),
      }],
      pending_data_element: None,
      is_complete: false,
    }
  }

  /// Returns whether the data set builder is complete, i.e. whether it has
  /// received the final [`P10Token::End`] token signalling the end of the
  /// incoming DICOM P10 tokens.
  ///
  pub fn is_complete(&self) -> bool {
    self.is_complete
  }

  /// Returns the File Preamble read by a data set builder, or an error if it
  /// has not yet been read. The File Preamble is always 128 bytes in size.
  ///
  /// The content of these bytes are application-defined, and are often unused
  /// and set to zero.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn file_preamble(&self) -> Result<&[u8; 128], ()> {
    match &self.file_preamble {
      Some(preamble) => Ok(preamble),
      None => Err(()),
    }
  }

  /// Returns the final data set constructed by a data set builder from the
  /// DICOM P10 tokens it has been fed, or an error if it has not yet been fully
  /// read.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn final_data_set(&mut self) -> Result<DataSet, ()> {
    let mut data_set = match (self.is_complete, self.location.as_mut_slice()) {
      (true, [BuilderLocation::RootDataSet { data_set }]) => {
        core::mem::take(data_set)
      }
      _ => return Err(()),
    };

    if let Some(file_meta_information) = self.file_meta_information.take() {
      data_set.merge(file_meta_information);
    }

    Ok(data_set)
  }

  /// Takes a data set builder that isn't yet complete, e.g. because an error
  /// was encountered reading the source of the P10 tokens it was being built
  /// from, and adds the necessary delimiter and end tokens so that it is
  /// considered complete and can have its final data set read out.
  ///
  /// This allows a partially built data set to be retrieved in its current
  /// state. This should never be needed when reading or constructing valid and
  /// complete DICOM P10 data.
  ///
  pub fn force_end(&mut self) {
    if self.is_complete {
      return;
    }

    self.pending_data_element = None;

    while let Some(location) = self.location.last() {
      match location {
        BuilderLocation::Sequence { tag, .. } => self
          .add_token(&P10Token::SequenceDelimiter { tag: *tag })
          .unwrap(),

        BuilderLocation::EncapsulatedPixelDataSequence { .. } => {
          self
            .add_token(&P10Token::SequenceDelimiter {
              tag: dictionary::PIXEL_DATA.tag,
            })
            .unwrap();
        }

        BuilderLocation::SequenceItem { .. } => {
          self.add_token(&P10Token::SequenceItemDelimiter).unwrap();
        }

        BuilderLocation::RootDataSet { .. } => {
          self.add_token(&P10Token::End).unwrap();
          return;
        }
      };
    }
  }

  /// Adds a new DICOM P10 token to a data set builder. This function is
  /// responsible for progressively constructing a data set from the tokens
  /// received, and also checks that the tokens being received are in a valid
  /// order.
  ///
  pub fn add_token(&mut self, token: &P10Token) -> Result<(), P10Error> {
    if self.is_complete {
      return Err(P10Error::TokenStreamInvalid {
        when: "Building data set".to_string(),
        details: "Token received after the token stream has ended".to_string(),
        token: token.clone(),
      });
    }

    // If there's a pending data element then it needs to be dealt with first as
    // the incoming token must be a DataElementValueBytes
    if self.pending_data_element.is_some() {
      return self.add_token_to_pending_data_element(token);
    }

    match (token, self.location.last()) {
      // Handle File Preamble token
      (P10Token::FilePreambleAndDICMPrefix { preamble }, _) => {
        self.file_preamble = Some(preamble.clone());
        Ok(())
      }

      // Handle File Meta Information token
      (P10Token::FileMetaInformation { data_set }, _) => {
        self.file_meta_information = Some(data_set.clone());
        Ok(())
      }

      // If a sequence is being read then add this token to it
      (token, Some(BuilderLocation::Sequence { .. })) => {
        self.add_token_to_sequence(token)
      }

      // If an encapsulated pixel data sequence is being read then add this
      // token to it
      (token, Some(BuilderLocation::EncapsulatedPixelDataSequence { .. })) => {
        self.add_token_to_encapsulated_pixel_data_sequence(token)
      }

      // Add this token to the current data set, which will be either the root
      // data set or an item in a sequence
      (token, _) => self.add_token_to_data_set(token),
    }
  }

  /// Ingests the next token when the data set builder's current location
  /// specifies a sequence.
  ///
  fn add_token_to_sequence(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10Error> {
    match (token, self.location.last()) {
      (
        P10Token::SequenceItemStart,
        Some(BuilderLocation::RootDataSet { .. }),
      )
      | (P10Token::SequenceItemStart, Some(BuilderLocation::Sequence { .. })) =>
      {
        self.location.push(BuilderLocation::SequenceItem {
          data_set: DataSet::new(),
        });

        Ok(())
      }

      (
        P10Token::SequenceDelimiter { .. },
        Some(BuilderLocation::Sequence { .. }),
      ) => {
        if let Some(BuilderLocation::Sequence { tag, items }) =
          self.location.pop()
        {
          let sequence = DataElementValue::new_sequence(items);
          self.insert_data_element_at_current_location(tag, sequence);
        }

        Ok(())
      }

      (token, _) => self.unexpected_token_error(token),
    }
  }

  /// Ingests the next token when the data set builder's current location
  /// specifies an encapsulated pixel data sequence.
  ///
  fn add_token_to_encapsulated_pixel_data_sequence(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10Error> {
    match (&token, self.location.last()) {
      (P10Token::PixelDataItem { .. }, _) => {
        self.pending_data_element = Some(PendingDataElement {
          tag: dictionary::ITEM.tag,
          vr: ValueRepresentation::OtherByteString,
          data: vec![],
        });

        Ok(())
      }

      (
        P10Token::SequenceDelimiter { .. },
        Some(BuilderLocation::EncapsulatedPixelDataSequence { .. }),
      ) => {
        if let Some(BuilderLocation::EncapsulatedPixelDataSequence {
          vr,
          items,
        }) = self.location.pop()
        {
          self.insert_data_element_at_current_location(
            dictionary::PIXEL_DATA.tag,
            DataElementValue::new_encapsulated_pixel_data_unchecked(vr, items),
          );
        }

        Ok(())
      }

      _ => self.unexpected_token_error(token),
    }
  }

  /// Ingests the next token when the data set builder's current location is in
  /// either the root data set or in an item that's part of a sequence.
  ///
  fn add_token_to_data_set(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10Error> {
    match token {
      // If this token is the start of a new data element then create a new
      // pending data element that will have its data filled in by subsequent
      // DataElementValueBytes tokens
      P10Token::DataElementHeader { tag, vr, .. } => {
        self.pending_data_element = Some(PendingDataElement {
          tag: *tag,
          vr: *vr,
          data: vec![],
        });

        Ok(())
      }

      // If this token indicates the start of a new sequence then update the
      // current location accordingly
      P10Token::SequenceStart { tag, vr } => {
        let new_location = match vr {
          ValueRepresentation::OtherByteString
          | ValueRepresentation::OtherWordString => {
            BuilderLocation::EncapsulatedPixelDataSequence {
              vr: *vr,
              items: vec![],
            }
          }

          _ => BuilderLocation::Sequence {
            tag: *tag,
            items: vec![],
          },
        };

        self.location.push(new_location);

        Ok(())
      }

      // If this token indicates the end of the current item then check that the
      // current location is in fact an item
      P10Token::SequenceItemDelimiter => match self.location.as_slice() {
        [
          ..,
          BuilderLocation::Sequence { .. },
          BuilderLocation::SequenceItem { .. },
        ] => {
          if let Some(BuilderLocation::SequenceItem { data_set }) =
            self.location.pop()
          {
            if let Some(BuilderLocation::Sequence { items, .. }) =
              self.location.last_mut()
            {
              items.push(data_set);
            }
          }

          Ok(())
        }

        _ => Err(P10Error::TokenStreamInvalid {
          when: "Building data set".to_string(),
          details: "Received sequence item delimiter token outside of an item"
            .to_string(),
          token: token.clone(),
        }),
      },

      // If this token indicates the end of the DICOM P10 tokens then mark the
      // builder as complete, so long as it's currently located in the root
      // data set
      P10Token::End => match self.location.as_slice() {
        [BuilderLocation::RootDataSet { .. }] => {
          self.is_complete = true;

          Ok(())
        }

        _ => Err(P10Error::TokenStreamInvalid {
          when: "Building data set".to_string(),
          details: "Received end token outside of the root data set"
            .to_string(),
          token: token.clone(),
        }),
      },

      token => self.unexpected_token_error(token),
    }
  }

  /// Ingests the next token when the data set builder has a pending data
  /// element that is expecting value bytes tokens containing its data.
  ///
  fn add_token_to_pending_data_element(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10Error> {
    match (token, self.pending_data_element.as_mut()) {
      (
        P10Token::DataElementValueBytes {
          data,
          bytes_remaining,
          ..
        },
        Some(pending_data_element),
      ) => {
        pending_data_element.data.push(data.clone());

        if *bytes_remaining == 0 {
          let tag = pending_data_element.tag;
          let value = build_final_data_element_value(
            tag,
            pending_data_element.vr,
            &pending_data_element.data,
          );

          self.insert_data_element_at_current_location(tag, value);

          self.pending_data_element = None;
        }

        Ok(())
      }

      (token, _) => self.unexpected_token_error(token),
    }
  }

  /// Inserts a new data element into the head of the given data set builder
  /// location and returns an updated location.
  ///
  fn insert_data_element_at_current_location(
    &mut self,
    tag: DataElementTag,
    value: DataElementValue,
  ) {
    match (self.location.as_mut_slice(), value.bytes()) {
      // Insert new data element into the root data set or current sequence item
      ([BuilderLocation::RootDataSet { data_set }], _)
      | ([.., BuilderLocation::SequenceItem { data_set }], _) => {
        data_set.insert(tag, value);
      }

      // Insert new data element into the current encapsulated pixel data
      // sequence
      (
        [
          ..,
          BuilderLocation::EncapsulatedPixelDataSequence { items, .. },
        ],
        Ok(bytes),
      ) => items.push(bytes.clone()),

      // Other locations aren't valid for insertion of a data element. This case
      // is not expected to be logically possible.
      _ => unreachable!(),
    };
  }

  /// The error returned when an unexpected DICOM P10 token is received.
  ///
  fn unexpected_token_error(&self, token: &P10Token) -> Result<(), P10Error> {
    Err(P10Error::TokenStreamInvalid {
      when: "Building data set".to_string(),
      details: format!(
        "Received unexpected P10 token at location: {}",
        location_to_string(&self.location),
      ),
      token: token.clone(),
    })
  }
}

/// Takes the tag, VR, and final bytes for a new data element and returns the
/// `DataElementValue` for it to insert into the active data set.
///
fn build_final_data_element_value(
  tag: DataElementTag,
  vr: ValueRepresentation,
  value_bytes: &[Rc<Vec<u8>>],
) -> DataElementValue {
  let value_length = value_bytes.iter().fold(0, |s, v| s + v.len());
  let mut bytes = Vec::with_capacity(value_length);

  // Concatenate all received bytes to get the bytes that are the final bytes
  // for the data element value
  for data in value_bytes.iter() {
    bytes.extend_from_slice(data);
  }

  let bytes = Rc::new(bytes);

  // Lookup table descriptors are a special case due to the non-standard way
  // their VR applies to their underlying bytes
  if dictionary::is_lut_descriptor_tag(tag) {
    DataElementValue::new_lookup_table_descriptor_unchecked(vr, bytes)
  } else {
    DataElementValue::new_binary_unchecked(vr, bytes)
  }
}

/// Converts a data set location to a human-readable string for error reporting
/// and debugging purposes.
///
fn location_to_string(location: &[BuilderLocation]) -> String {
  let mut result = vec![];

  for item in location {
    result.push(match item {
      BuilderLocation::RootDataSet { .. } => "RootDataSet".to_string(),
      BuilderLocation::Sequence { tag, .. } => format!("Sequence{}", tag),
      BuilderLocation::SequenceItem { .. } => "SequenceItem".to_string(),
      BuilderLocation::EncapsulatedPixelDataSequence { .. } => {
        "EncapsulatedPixelDataSequence".to_string()
      }
    });
  }

  result.join(".")
}
