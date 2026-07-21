//! Reads and writes waveform data, such as ECGs, stored in DICOM data sets,
//! including decoding and encoding of per-channel sample data.
//!
//! Ref: PS3.3 C.10.9.

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

pub mod iods;
pub mod transforms;

mod decode;
mod encode;
mod p10_waveform_chunk_writer;
mod waveform_chunk;

pub use decode::WaveformDecodeError;
pub use encode::WaveformEncodeError;
pub use iods::{
  ChannelDefinition, ChannelStatus, CodedConcept, WaveformBitsAllocated,
  WaveformModule, WaveformMultiplexGroup, WaveformOriginality,
  WaveformSampleInterpretation,
};
pub use p10_waveform_chunk_writer::{
  P10WaveformChunkWriter, P10WaveformChunkWriterError,
};
pub use transforms::{
  P10WaveformChunkTransform, P10WaveformChunkTransformError,
};
pub use waveform_chunk::WaveformChunk;

use dcmfx_core::{DataError, DataSet, DataSetPath, IodModule, dictionary};
use dcmfx_p10::DataSetP10Extensions;

/// Adds functions to [`DataSet`] for reading its waveform data.
///
pub trait DataSetWaveformExtensions {
  /// Reads the Waveform module from this data set.
  ///
  /// Ref: PS3.3 C.10.9.
  ///
  fn waveform_module(&self) -> Result<WaveformModule, DataError>;

  /// Returns the chunks of waveform data in this data set. Each chunk
  /// references the waveform multiplex group that its sample sets belong to.
  ///
  fn get_waveform_chunks(
    &self,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError>;
}

impl DataSetWaveformExtensions for DataSet {
  fn waveform_module(&self) -> Result<WaveformModule, DataError> {
    WaveformModule::from_data_set(self)
  }

  fn get_waveform_chunks(
    &self,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError> {
    // Create a new data set containing only the Waveform Sequence. This
    // avoids calling DataSet::to_p10_token_stream() on the whole data set.
    let mut data_set = DataSet::new();
    if let Ok(value) = self.get_value(dictionary::WAVEFORM_SEQUENCE.tag) {
      // Error when the Waveform Sequence value isn't a sequence, as it would
      // otherwise be silently ignored by the waveform chunk transform
      value.sequence_items().map_err(|e| {
        P10WaveformChunkTransformError::DataError(e.with_path(
          &DataSetPath::new_with_data_element(
            dictionary::WAVEFORM_SEQUENCE.tag,
          ),
        ))
      })?;

      data_set.insert(dictionary::WAVEFORM_SEQUENCE.tag, value.clone());
    }

    // Pass the cut down data set through a waveform chunk transform and
    // collect all emitted chunks
    let mut waveform_chunk_transform = P10WaveformChunkTransform::new();
    let mut chunks = vec![];
    data_set.to_p10_token_stream(&mut |token| {
      chunks.extend(waveform_chunk_transform.add_token(&token)?);
      Ok(())
    })?;

    Ok(chunks)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::DataElementValue;

  #[test]
  fn get_waveform_chunks_errors_when_waveform_sequence_is_not_a_sequence() {
    let mut data_set = DataSet::new();
    data_set.insert(
      dictionary::WAVEFORM_SEQUENCE.tag,
      DataElementValue::new_unsigned_short(&[0]).unwrap(),
    );

    assert!(matches!(
      data_set.get_waveform_chunks(),
      Err(P10WaveformChunkTransformError::DataError(_))
    ));
  }
}
