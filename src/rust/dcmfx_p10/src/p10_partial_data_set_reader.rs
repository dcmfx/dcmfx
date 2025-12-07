//! Reads a specific set of data elements out of incoming chunks of binary
//! DICOM P10 data.
//!
//! This is a streaming alternative to [`crate::read_stream_partial`] for when
//! the DICOM P10 data isn't available as a stream that can be read from, e.g.
//! when it is being consumed by something else at the same time.

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

use bytes::Bytes;

use dcmfx_core::{DataElementTag, DataSet};

use crate::{
  DataSetBuilder, P10Error, P10FilterTransform, P10ReadConfig, P10ReadContext,
  P10Token,
};

/// Reads a specific set of data elements at the root of the main data set out
/// of DICOM P10 data. Raw DICOM P10 data with [`Self::write_bytes`], and once
/// [`Self::is_complete`] returns true no further data needs to be provided
/// and an output [`DataSet`] is made available.
///
pub struct P10PartialDataSetReader {
  tags: Vec<DataElementTag>,
  largest_tag: DataElementTag,
  read_context: P10ReadContext,
  filter: P10FilterTransform,
  data_set_builder: DataSetBuilder,
  is_complete: bool,
}

impl P10PartialDataSetReader {
  /// Creates a new partial data set reader that reads the specified data
  /// elements at the root of the main data set, if present.
  ///
  pub fn new(
    tags: &[DataElementTag],
    config: Option<P10ReadConfig>,
  ) -> P10PartialDataSetReader {
    // Find the largest data element tag being read
    let largest_tag =
      tags.iter().max().cloned().unwrap_or(DataElementTag::ZERO);

    // Create filter transform that only allows the specified root tags
    let filter = {
      let tags = tags.to_vec();
      P10FilterTransform::new(Box::new(
        move |tag, _vr, _length, path| -> bool {
          !path.is_root() || tags.contains(&tag)
        },
      ))
    };

    P10PartialDataSetReader {
      tags: tags.to_vec(),
      largest_tag,
      read_context: P10ReadContext::new(config),
      filter,
      data_set_builder: DataSetBuilder::new(),
      is_complete: false,
    }
  }

  /// Returns whether all of the requested data elements have been read from the
  /// DICOM P10 data provided through [`Self::write_bytes`]. Once this returns
  /// true there is no need to call [`Self::write_bytes`] again.
  ///
  pub fn is_complete(&self) -> bool {
    self.is_complete
  }

  /// Adds the next chunk of raw DICOM P10 data to this partial data set reader.
  /// The `done` argument specifies whether the end of the DICOM P10 data has
  /// been reached.
  ///
  /// Passing further data once [`Self::is_complete`] returns true does nothing.
  ///
  pub fn write_bytes(
    &mut self,
    bytes: Bytes,
    done: bool,
  ) -> Result<(), P10Error> {
    if self.is_complete {
      return Ok(());
    }

    self.read_context.write_bytes(bytes, done)?;

    loop {
      let tokens = match self.read_context.read_tokens() {
        Ok(tokens) => tokens,

        // Once the read context needs more data this chunk has been fully
        // consumed
        Err(P10Error::DataRequired { .. }) => return Ok(()),

        Err(e) => return Err(e),
      };

      for token in tokens.iter() {
        if self.filter.add_token(token)? {
          self.data_set_builder.add_token(token)?;
        }

        match token {
          P10Token::DataElementHeader { tag, path, .. }
          | P10Token::SequenceStart { tag, path, .. } => {
            if *tag > self.largest_tag && path.is_root() {
              self.is_complete = true;
              return Ok(());
            }
          }

          P10Token::End => {
            self.is_complete = true;
            return Ok(());
          }

          _ => (),
        }
      }
    }
  }

  /// Returns the data set containing the requested data elements that were
  /// present in the DICOM P10 data passed to [`Self::write_bytes`]. This is
  /// typically called only once [`Self::is_complete`] returns true.
  ///
  pub fn into_data_set(mut self) -> DataSet {
    self.data_set_builder.force_end();
    let mut data_set = self.data_set_builder.final_data_set().unwrap();

    // Exclude File Meta Information tags unless they were explicitly requested
    data_set.retain(|tag, _value| {
      !tag.is_file_meta_information() || self.tags.contains(&tag)
    });

    data_set
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::dictionary;

  #[test]
  fn write_bytes_in_chunks_test() {
    let path = "../../../test/assets/pydicom/test_files/693_J2KI.dcm";
    let bytes = std::fs::read(path).unwrap();

    let tags = [dictionary::ROWS.tag, dictionary::COLUMNS.tag];
    let mut reader = P10PartialDataSetReader::new(&tags, None);

    for chunk in bytes.chunks(100) {
      reader
        .write_bytes(Bytes::copy_from_slice(chunk), false)
        .unwrap();

      if reader.is_complete() {
        break;
      }
    }

    assert!(reader.is_complete());
    assert_eq!(reader.into_data_set().tags(), tags.to_vec());
  }
}
