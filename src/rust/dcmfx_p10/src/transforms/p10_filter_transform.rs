use dcmfx_core::{DataElementTag, ValueRepresentation, dictionary};

use crate::P10Token;

/// Transform that applies a data element filter to a stream of DICOM P10
/// tokens.
///
pub struct P10FilterTransform {
  predicate: Box<PredicateFunction>,
  location: Vec<LocationEntry>,
}

pub struct LocationEntry {
  #[allow(dead_code)]
  tag: DataElementTag,
  filter_result: bool,
}

type PredicateFunction =
  dyn FnMut(DataElementTag, ValueRepresentation, &[LocationEntry]) -> bool;

impl P10FilterTransform {
  /// Creates a new filter transform for filtering a stream of DICOM P10 tokens.
  ///
  /// The predicate function is called as tokens are added to the context, and
  /// only those data elements that return `true` from the predicate function
  /// will pass through this filter transform.
  ///
  pub fn new(predicate: Box<PredicateFunction>) -> Self {
    Self {
      predicate,
      location: vec![],
    }
  }

  /// Returns whether the current position of the P10 filter context is the root
  /// data set, i.e. there are no nested sequences currently active.
  ///
  pub fn is_at_root(&self) -> bool {
    self.location.is_empty()
  }

  /// Adds the next token to the P10 filter transform and returns whether it
  /// should be included in the filtered token stream.
  ///
  pub fn add_token(&mut self, token: &P10Token) -> bool {
    match token {
      // If this is a new sequence or data element then run the predicate
      // function to see if it passes the filter, then add it to the location
      P10Token::SequenceStart { tag, vr }
      | P10Token::DataElementHeader { tag, vr, .. } => {
        // The predicate function is skipped if a parent has already been
        // filtered out
        let filter_result = match self.location.as_slice() {
          []
          | [
            ..,
            LocationEntry {
              filter_result: true,
              ..
            },
          ] => (self.predicate)(*tag, *vr, &self.location),

          _ => false,
        };

        self.location.push(LocationEntry {
          tag: *tag,
          filter_result,
        });

        filter_result
      }

      // If this is a new pixel data item then add it to the location
      P10Token::PixelDataItem { .. } => {
        let filter_result = match self.location.last() {
          Some(LocationEntry { filter_result, .. }) => *filter_result,
          None => true,
        };

        self.location.push(LocationEntry {
          tag: dictionary::ITEM.tag,
          filter_result,
        });

        filter_result
      }

      // Detect the end of the entry at the head of the location and pop it off
      P10Token::SequenceDelimiter { .. }
      | P10Token::DataElementValueBytes {
        bytes_remaining: 0, ..
      } => {
        let filter_result = match self.location.last() {
          Some(LocationEntry { filter_result, .. }) => *filter_result,
          None => true,
        };

        self.location.pop();

        filter_result
      }

      _ => {
        match self.location.last() {
          // If tokens are currently being filtered out then swallow this one
          Some(LocationEntry { filter_result, .. }) => *filter_result,

          // Otherwise this token passes through the filter
          None => true,
        }
      }
    }
  }
}
