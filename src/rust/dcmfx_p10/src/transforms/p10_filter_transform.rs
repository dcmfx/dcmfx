#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{DataElementTag, DataSetPath, ValueRepresentation};

use crate::{P10Error, P10Token};

/// Transform that applies a data element filter to a stream of DICOM P10
/// tokens. Incoming data elements are passed to a predicate function that
/// determines whether they should be present in the output DICOM P10 token
/// stream.
///
pub struct P10FilterTransform {
  predicate: Box<PredicateFunction>,
  path: DataSetPath,
  path_filter_results: Vec<bool>,
}

/// Defines a function called by a [`P10FilterTransform`] that determines
/// whether a data element should pass through the filter.
///
pub type PredicateFunction = dyn FnMut(
  DataElementTag,
  ValueRepresentation,
  Option<u32>,
  &DataSetPath,
) -> bool;

impl P10FilterTransform {
  /// Creates a new filter transform for filtering a stream of DICOM P10 tokens.
  ///
  /// The predicate function is called as tokens are added to the context, and
  /// only those data elements that return `true` from the predicate function
  /// will pass through the filter.
  ///
  pub fn new(predicate: Box<PredicateFunction>) -> Self {
    Self {
      predicate,
      path: DataSetPath::new(),
      path_filter_results: vec![],
    }
  }

  /// Returns whether the current position of the P10 filter context is the root
  /// data set, i.e. there are no nested sequences currently active.
  ///
  pub fn is_at_root(&self) -> bool {
    self.path.is_empty()
  }

  /// Adds the next token to the P10 filter transform and returns whether it
  /// should be included in the filtered token stream.
  ///
  pub fn add_token(&mut self, token: &P10Token) -> Result<bool, P10Error> {
    let current_filter_state =
      *self.path_filter_results.last().unwrap_or(&true);

    let map_data_set_path_error = |details: String| -> P10Error {
      P10Error::TokenStreamInvalid {
        when: "Filtering P10 token stream".to_string(),
        details,
        token: token.clone(),
      }
    };

    let mut run_predicate =
      |tag, vr, length: Option<u32>| -> Result<bool, P10Error> {
        let filter_result = match self.path_filter_results.as_slice() {
          [] | [.., true] => (self.predicate)(tag, vr, length, &self.path),

          // The predicate function is skipped if a parent has already been
          // filtered out
          _ => false,
        };

        self
          .path
          .add_data_element(tag)
          .map_err(map_data_set_path_error)?;

        self.path_filter_results.push(filter_result);

        Ok(filter_result)
      };

    match token {
      // If this is a new sequence or data element then run the predicate
      // function to see if it passes the filter
      P10Token::SequenceStart { tag, vr } => run_predicate(*tag, *vr, None),
      P10Token::DataElementHeader { tag, vr, length } => {
        run_predicate(*tag, *vr, Some(*length))
      }

      P10Token::SequenceItemStart { index } => {
        self
          .path
          .add_sequence_item(*index)
          .map_err(map_data_set_path_error)?;

        Ok(current_filter_state)
      }

      P10Token::SequenceItemDelimiter => {
        self.path.pop().map_err(map_data_set_path_error)?;

        Ok(current_filter_state)
      }

      // If this is a new pixel data item then add it to the location
      P10Token::PixelDataItem { index, .. } => {
        self
          .path
          .add_sequence_item(*index)
          .map_err(map_data_set_path_error)?;

        self.path_filter_results.push(current_filter_state);

        Ok(current_filter_state)
      }

      // Detect the end of the entry at the head of the location and pop it off
      P10Token::SequenceDelimiter { .. }
      | P10Token::DataElementValueBytes {
        bytes_remaining: 0, ..
      } => {
        self.path.pop().map_err(map_data_set_path_error)?;
        self.path_filter_results.pop().unwrap();

        Ok(current_filter_state)
      }

      _ => Ok(current_filter_state),
    }
  }
}
