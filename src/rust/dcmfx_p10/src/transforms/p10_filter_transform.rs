#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, string::ToString, vec, vec::Vec};

use dcmfx_core::{DataElementTag, DataSetPath, ValueRepresentation};

use crate::{P10Error, P10Token};

/// Transform that applies a data element filter to a stream of DICOM P10
/// tokens. Incoming data elements are passed to a predicate function that
/// determines whether they should be present in the output DICOM P10 token
/// stream.
///
pub struct P10FilterTransform {
  predicate: Box<PredicateFunction>,
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
  /// The predicate function is called as tokens are added to the transform, and
  /// only those data elements that return `true` from the predicate function
  /// will pass through the filter.
  ///
  pub fn new(predicate: Box<PredicateFunction>) -> Self {
    Self {
      predicate,
      path_filter_results: vec![],
    }
  }

  /// Returns whether the current position of the P10 filter transform is the
  /// root data set, i.e. there are no nested sequences currently active.
  ///
  pub fn is_at_root(&self) -> bool {
    self.path_filter_results.len() <= 1
  }

  /// Adds the next token to the P10 filter transform and returns whether it
  /// should be included in the filtered token stream.
  ///
  pub fn add_token(&mut self, token: &P10Token) -> Result<bool, P10Error> {
    let current_filter_state =
      *self.path_filter_results.last().unwrap_or(&true);

    let mut run_predicate = |tag, vr, length, path| -> Result<bool, P10Error> {
      let filter_result = match self.path_filter_results.as_slice() {
        [] | [.., true] => (self.predicate)(tag, vr, length, path),

        // The predicate function is skipped if a parent has already been
        // filtered out
        _ => false,
      };

      self.path_filter_results.push(filter_result);

      Ok(filter_result)
    };

    match token {
      P10Token::FilePreambleAndDICMPrefix { .. }
      | P10Token::FileMetaInformation { .. } => Ok(true),

      P10Token::SequenceStart { tag, vr, path } => {
        run_predicate(*tag, *vr, None, path)
      }

      P10Token::SequenceDelimiter { .. } => {
        self
          .path_filter_results
          .pop()
          .ok_or(P10Error::TokenStreamInvalid {
            when: "Adding token to filter transform".to_string(),
            details: "Sequence delimiter received when current path is empty"
              .to_string(),
            token: token.clone(),
          })?;

        Ok(current_filter_state)
      }

      P10Token::SequenceItemStart { .. } => {
        self.path_filter_results.push(current_filter_state);

        Ok(current_filter_state)
      }

      P10Token::SequenceItemDelimiter => {
        self
          .path_filter_results
          .pop()
          .ok_or(P10Error::TokenStreamInvalid {
            when: "Adding token to filter transform".to_string(),
            details:
              "Sequence item delimiter received when current path is empty"
                .to_string(),
            token: token.clone(),
          })?;

        Ok(current_filter_state)
      }

      P10Token::DataElementHeader {
        tag,
        vr,
        length,
        path,
      } => run_predicate(*tag, *vr, Some(*length), path),

      P10Token::DataElementValueBytes {
        bytes_remaining, ..
      } => {
        if *bytes_remaining == 0 {
          self.path_filter_results.pop().ok_or(
            P10Error::TokenStreamInvalid {
              when: "Adding token to filter transform".to_string(),
              details: "Data element bytes ended when current path is empty"
                .to_string(),
              token: token.clone(),
            },
          )?;
        }

        Ok(current_filter_state)
      }

      P10Token::PixelDataItem { .. } => {
        self.path_filter_results.push(current_filter_state);

        Ok(current_filter_state)
      }

      P10Token::End => Ok(true),
    }
  }
}
