//! Extracts chunks of waveform data from a stream of DICOM P10 tokens.

#[cfg(not(feature = "std"))]
use alloc::{
  boxed::Box,
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{
  DataError, DataSetPath, DcmfxError, Rc, RcByteSlice, dictionary,
};
use dcmfx_p10::{DataSetBuilder, P10Error, P10FilterTransform, P10Token};

use crate::{WaveformChunk, WaveformMultiplexGroup};

/// This transform takes a stream of DICOM P10 tokens and emits the waveform
/// data it contains as a stream of [`WaveformChunk`]s, where each chunk holds
/// consecutive whole sample sets of a single waveform multiplex group.
///
/// Waveform data is stored with its channels interleaved, so each chunk holds
/// the samples of all channels for a range of the recording. Memory usage is
/// bounded by the size of the incoming tokens rather than the size of the
/// waveform data, allowing waveforms of unbounded size, e.g. multi-hour
/// ambulatory ECG recordings, to be processed in a stream.
///
pub struct P10WaveformChunkTransform {
  // Filter that matches the root '(5400,0100) Waveform Sequence' data element
  // and everything inside it
  waveform_sequence_filter: P10FilterTransform,

  // The current depth of nested sequences inside the Waveform Sequence. A
  // depth of one means the current position is directly inside an item of the
  // Waveform Sequence, i.e. inside a multiplex group.
  sequence_depth: usize,

  // Whether the Waveform Sequence has been fully read
  is_complete: bool,

  // The index of the current item of the Waveform Sequence
  multiplex_group_index: usize,

  // Gathers the data elements of the current multiplex group that precede its
  // '(5400,1010) Waveform Data'
  multiplex_group_builder: Option<DataSetBuilder>,

  // The current multiplex group. This is set while its '(5400,1010) Waveform
  // Data' is being read, and is shared by the emitted chunks.
  multiplex_group: Option<Rc<WaveformMultiplexGroup>>,

  // The index of the next sample set to be emitted for the current multiplex
  // group
  sample_offset: u64,

  // Waveform data bytes that don't yet form a whole sample set
  pending_bytes: Vec<u8>,
}

/// An error that occurred in the process of extracting chunks of waveform
/// data from a stream of DICOM P10 tokens.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10WaveformChunkTransformError {
  /// An error that occurred when adding a P10 token. This can happen when the
  /// stream of DICOM P10 tokens is invalid.
  P10Error(P10Error),

  /// An error that occurred when reading the data from the data elements in
  /// the stream of DICOM P10 tokens.
  DataError(DataError),
}

impl core::fmt::Display for P10WaveformChunkTransformError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::P10Error(e) => e.fmt(f),
      Self::DataError(e) => e.fmt(f),
    }
  }
}

impl DcmfxError for P10WaveformChunkTransformError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::P10Error(e) => e.to_lines(task_description),
      Self::DataError(e) => e.to_lines(task_description),
    }
  }
}

impl P10WaveformChunkTransform {
  /// Creates a new P10 waveform chunk transform to extract chunks of waveform
  /// data from a stream of DICOM P10 tokens.
  ///
  pub fn new() -> Self {
    // Match the root Waveform Sequence and everything inside it. The
    // predicate is not called for the content of root data elements that
    // don't match, so matching all non-root data elements only matches those
    // inside the Waveform Sequence.
    let waveform_sequence_filter =
      P10FilterTransform::new(Box::new(|tag, _vr, _length, path| {
        !path.is_root() || tag == dictionary::WAVEFORM_SEQUENCE.tag
      }));

    Self {
      waveform_sequence_filter,
      sequence_depth: 0,
      is_complete: false,
      multiplex_group_index: 0,
      multiplex_group_builder: None,
      multiplex_group: None,
      sample_offset: 0,
      pending_bytes: vec![],
    }
  }

  /// Adds the next DICOM P10 token, returning any chunks of waveform data
  /// that are now available.
  ///
  pub fn add_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError> {
    if self.is_complete || token.is_header_token() {
      return Ok(vec![]);
    }

    if self
      .waveform_sequence_filter
      .add_token(token)
      .map_err(P10WaveformChunkTransformError::P10Error)?
    {
      self.process_next_waveform_sequence_token(token)
    } else {
      Ok(vec![])
    }
  }

  fn process_next_waveform_sequence_token(
    &mut self,
    token: &P10Token,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError> {
    match token {
      // The start of the Waveform Sequence itself, or of a sequence nested
      // inside one of its multiplex groups
      P10Token::SequenceStart { .. } => {
        if self.sequence_depth > 0 {
          self.add_token_to_multiplex_group_builder(token)?;
        }

        self.sequence_depth += 1;

        Ok(vec![])
      }

      P10Token::SequenceDelimiter { .. } => {
        self.sequence_depth = self.sequence_depth.saturating_sub(1);

        if self.sequence_depth == 0 {
          self.is_complete = true;
        } else {
          self.add_token_to_multiplex_group_builder(token)?;
        }

        Ok(vec![])
      }

      // The start of the next multiplex group, or of an item in a sequence
      // nested inside the current multiplex group
      P10Token::SequenceItemStart { .. } => {
        if self.sequence_depth == 1 {
          self.multiplex_group_builder = Some(DataSetBuilder::new());
          self.multiplex_group = None;
          self.sample_offset = 0;
          self.pending_bytes.clear();
        } else {
          self.add_token_to_multiplex_group_builder(token)?;
        }

        Ok(vec![])
      }

      P10Token::SequenceItemDelimiter => {
        if self.sequence_depth == 1 {
          self.end_multiplex_group()?;
        } else {
          self.add_token_to_multiplex_group_builder(token)?;
        }

        Ok(vec![])
      }

      P10Token::DataElementHeader { tag, length, .. } => {
        if self.sequence_depth == 1 && *tag == dictionary::WAVEFORM_DATA.tag {
          self.start_waveform_data(*length)?;
        } else {
          self.add_token_to_multiplex_group_builder(token)?;
        }

        Ok(vec![])
      }

      P10Token::DataElementValueBytes {
        tag,
        data,
        bytes_remaining,
        ..
      } => {
        if self.multiplex_group.is_some()
          && *tag == dictionary::WAVEFORM_DATA.tag
        {
          self.add_waveform_data_bytes(data, *bytes_remaining)
        } else {
          self.add_token_to_multiplex_group_builder(token)?;

          Ok(vec![])
        }
      }

      _ => Ok(vec![]),
    }
  }

  fn add_token_to_multiplex_group_builder(
    &mut self,
    token: &P10Token,
  ) -> Result<(), P10WaveformChunkTransformError> {
    if let Some(multiplex_group_builder) = self.multiplex_group_builder.as_mut()
    {
      multiplex_group_builder
        .add_token(token)
        .map_err(P10WaveformChunkTransformError::P10Error)?;
    }

    Ok(())
  }

  /// Handles the start of a multiplex group's '(5400,1010) Waveform Data'
  /// data element. All of the multiplex group's other data elements have now
  /// been received, so the multiplex group is constructed, and the length of
  /// the waveform data is validated prior to any of it being received.
  ///
  fn start_waveform_data(
    &mut self,
    length: u32,
  ) -> Result<(), P10WaveformChunkTransformError> {
    let Some(mut multiplex_group_builder) = self.multiplex_group_builder.take()
    else {
      return Err(P10WaveformChunkTransformError::DataError(
        DataError::new_value_invalid(
          "Multiplex group has multiple waveform data data elements"
            .to_string(),
        )
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::WAVEFORM_DATA.tag,
        )),
      ));
    };

    multiplex_group_builder.force_end();
    let item_data_set = multiplex_group_builder.final_data_set().unwrap();

    let multiplex_group = WaveformMultiplexGroup::from_data_set(&item_data_set)
      .map_err(P10WaveformChunkTransformError::DataError)?;

    // Check the length of the waveform data, allowing for a single trailing
    // padding byte as binary data elements must have even length
    let expected_length = multiplex_group.waveform_data_length();
    let actual_length = u64::from(length);

    if actual_length != expected_length
      && actual_length != expected_length.next_multiple_of(2)
    {
      return Err(P10WaveformChunkTransformError::DataError(
        DataError::new_value_invalid(format!(
          "Waveform data length of {actual_length} bytes is invalid, \
           expected {expected_length} bytes"
        ))
        .with_path(&DataSetPath::new_with_data_element(
          dictionary::WAVEFORM_DATA.tag,
        )),
      ));
    }

    self.multiplex_group = Some(Rc::new(multiplex_group));

    Ok(())
  }

  /// Adds the next bytes of the current multiplex group's '(5400,1010)
  /// Waveform Data' and emits the whole sample sets that are now available as
  /// a chunk. Bytes that don't yet form a whole sample set are carried over
  /// to the next call.
  ///
  fn add_waveform_data_bytes(
    &mut self,
    data: &RcByteSlice,
    bytes_remaining: u32,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError> {
    let Some(multiplex_group) = self.multiplex_group.as_ref() else {
      return Ok(vec![]);
    };

    let sample_set_size = multiplex_group.sample_set_size();

    let mut chunks = vec![];

    if sample_set_size > 0 {
      let remaining_sample_sets = usize::try_from(
        u64::from(multiplex_group.number_of_samples()) - self.sample_offset,
      )
      .unwrap_or(usize::MAX);

      // Determine the bytes of whole sample sets now available, avoiding a
      // copy of the incoming data when it starts on a sample set boundary
      let chunk_data = if self.pending_bytes.is_empty() {
        let sample_sets =
          (data.len() / sample_set_size).min(remaining_sample_sets);
        let length = sample_sets * sample_set_size;

        self.pending_bytes.extend_from_slice(&data[length..]);

        if length > 0 {
          Some(data.take(length))
        } else {
          None
        }
      } else {
        self.pending_bytes.extend_from_slice(data);

        let sample_sets = (self.pending_bytes.len() / sample_set_size)
          .min(remaining_sample_sets);
        let length = sample_sets * sample_set_size;

        if length > 0 {
          let rest = self.pending_bytes.split_off(length);
          let bytes = core::mem::replace(&mut self.pending_bytes, rest);

          Some(bytes.into())
        } else {
          None
        }
      };

      if let Some(chunk_data) = chunk_data {
        let sample_count = (chunk_data.len() / sample_set_size) as u32;

        chunks.push(WaveformChunk::new(
          multiplex_group.clone(),
          self.multiplex_group_index,
          self.sample_offset,
          sample_count,
          chunk_data,
        ));

        self.sample_offset += u64::from(sample_count);
      }
    }

    // On the final bytes of the waveform data, check that all sample sets
    // were emitted, allowing for a single trailing padding byte as binary
    // data elements must have even length
    if bytes_remaining == 0 {
      let multiplex_group = self.multiplex_group.take().unwrap();

      if self.sample_offset != u64::from(multiplex_group.number_of_samples())
        || self.pending_bytes.len() > 1
      {
        return Err(P10WaveformChunkTransformError::DataError(
          DataError::new_value_invalid(
            "Waveform data does not hold a whole number of sample sets"
              .to_string(),
          )
          .with_path(&DataSetPath::new_with_data_element(
            dictionary::WAVEFORM_DATA.tag,
          )),
        ));
      }

      self.pending_bytes.clear();
    }

    Ok(chunks)
  }

  /// Handles the end of an item of the Waveform Sequence.
  ///
  fn end_multiplex_group(
    &mut self,
  ) -> Result<(), P10WaveformChunkTransformError> {
    // The multiplex group must have contained a '(5400,1010) Waveform Data'
    // data element
    if self.multiplex_group_builder.take().is_some() {
      return Err(P10WaveformChunkTransformError::DataError(
        DataError::new_tag_not_present().with_path(
          &DataSetPath::new_with_data_element(dictionary::WAVEFORM_DATA.tag),
        ),
      ));
    }

    self.multiplex_group_index += 1;

    Ok(())
  }
}

impl Default for P10WaveformChunkTransform {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_core::{DataSet, ValueRepresentation};
  use dcmfx_p10::DataSetP10Extensions;

  use crate::iods::{
    ChannelDefinition, CodedConcept, WaveformBitsAllocated,
    WaveformOriginality, WaveformSampleInterpretation,
  };

  fn test_channel() -> ChannelDefinition {
    ChannelDefinition {
      waveform_channel_number: None,
      label: None,
      status: vec![],
      source: CodedConcept {
        code_value: Some("5.6.3-9-1".to_string()),
        coding_scheme_designator: Some("SCPECG".to_string()),
        coding_scheme_version: None,
        code_meaning: Some("Lead I (Einthoven)".to_string()),
      },
      derivation_description: None,
      sensitivity: None,
      sensitivity_units: None,
      sensitivity_correction_factor: None,
      baseline: None,
      time_skew: None,
      sample_skew: None,
      offset: None,
      bits_stored: 16,
      amplifier_type: None,
      filter_low_frequency: None,
      filter_high_frequency: None,
      notch_filter_frequency: None,
      notch_filter_bandwidth: None,
      minimum_value: None,
      maximum_value: None,
    }
  }

  fn test_multiplex_group(
    number_of_channels: usize,
    number_of_samples: u32,
  ) -> WaveformMultiplexGroup {
    WaveformMultiplexGroup::new(
      WaveformOriginality::Original,
      number_of_samples,
      500.0,
      None,
      None,
      None,
      None,
      None,
      (0..number_of_channels).map(|_| test_channel()).collect(),
      WaveformBitsAllocated::Sixteen,
      WaveformSampleInterpretation::SignedShort,
      None,
    )
    .unwrap()
  }

  /// Encodes signed 16-bit sample sets, given as `samples[channel][sample]`,
  /// into channel-interleaved little-endian waveform data bytes.
  ///
  fn interleave_i16_samples(samples: &[Vec<i64>]) -> Vec<u8> {
    let mut bytes = vec![];

    for sample_index in 0..samples[0].len() {
      for channel_samples in samples.iter() {
        bytes.extend_from_slice(
          &(channel_samples[sample_index] as i16).to_le_bytes(),
        );
      }
    }

    bytes
  }

  /// Builds a data set holding a Waveform Sequence with an item for each of
  /// the given multiplex groups and its waveform data bytes.
  ///
  fn test_data_set(
    multiplex_groups: &[(&WaveformMultiplexGroup, Vec<u8>)],
  ) -> DataSet {
    let items = multiplex_groups
      .iter()
      .map(|(multiplex_group, waveform_data)| {
        let mut item = multiplex_group.to_data_set().unwrap();
        item
          .insert_binary_value(
            dictionary::WAVEFORM_DATA.tag,
            ValueRepresentation::OtherWordString,
            waveform_data.clone().into(),
          )
          .unwrap();
        item
      })
      .collect();

    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(&dictionary::WAVEFORM_SEQUENCE, items)
      .unwrap();

    data_set
  }

  fn add_tokens_for_data_set(
    data_set: &DataSet,
    transform: &mut P10WaveformChunkTransform,
  ) -> Result<Vec<WaveformChunk>, P10WaveformChunkTransformError> {
    let mut chunks = vec![];

    data_set.to_p10_token_stream(&mut |token| {
      chunks.extend(transform.add_token(&token)?);
      Ok(())
    })?;

    Ok(chunks)
  }

  #[test]
  fn streams_multiplex_groups_as_chunks() {
    let group_0 = test_multiplex_group(2, 5);
    let group_0_samples = [vec![1, 2, 3, 4, 5], vec![6, 7, 8, 9, 10]];

    let group_1 = test_multiplex_group(1, 2);
    let group_1_samples = [vec![-1, -2]];

    let data_set = test_data_set(&[
      (&group_0, interleave_i16_samples(&group_0_samples)),
      (&group_1, interleave_i16_samples(&group_1_samples)),
    ]);

    let mut transform = P10WaveformChunkTransform::new();
    let chunks = add_tokens_for_data_set(&data_set, &mut transform).unwrap();

    // The waveform data is small so arrives in a single token per multiplex
    // group, and hence in a single chunk per multiplex group
    assert_eq!(chunks.len(), 2);

    assert_eq!(chunks[0].multiplex_group().as_ref(), &group_0);
    assert_eq!(chunks[0].multiplex_group_index(), 0);
    assert_eq!(chunks[0].sample_offset(), 0);
    assert_eq!(chunks[0].number_of_samples(), 5);
    assert_eq!(chunks[0].channel_samples(), Ok(group_0_samples.to_vec()));

    assert_eq!(chunks[1].multiplex_group().as_ref(), &group_1);
    assert_eq!(chunks[1].multiplex_group_index(), 1);
    assert_eq!(chunks[1].sample_offset(), 0);
    assert_eq!(chunks[1].number_of_samples(), 2);
    assert_eq!(chunks[1].channel_samples(), Ok(group_1_samples.to_vec()));
  }

  #[test]
  fn streams_chunks_across_split_value_bytes_tokens() {
    let samples: [Vec<i64>; 2] =
      [(0..100).collect(), (0..100).map(|i| -i).collect()];

    let group = test_multiplex_group(2, 100);
    let data_set = test_data_set(&[(&group, interleave_i16_samples(&samples))]);

    // Stream the data set's tokens, splitting the waveform data value bytes
    // into seven-byte tokens so that sample sets are split across tokens
    let mut transform = P10WaveformChunkTransform::new();
    let mut chunks = vec![];
    data_set
      .to_p10_token_stream(&mut |token| {
        match &token {
          P10Token::DataElementValueBytes {
            tag,
            vr,
            data,
            bytes_remaining,
          } if *tag == dictionary::WAVEFORM_DATA.tag => {
            let mut offset = 0;
            while offset < data.len() {
              let end = (offset + 7).min(data.len());

              let token = P10Token::DataElementValueBytes {
                tag: *tag,
                vr: *vr,
                data: data.slice(offset, end),
                bytes_remaining: bytes_remaining + (data.len() - end) as u32,
              };

              chunks.extend(transform.add_token(&token)?);

              offset = end;
            }
          }

          _ => chunks.extend(transform.add_token(&token)?),
        }

        Ok::<(), P10WaveformChunkTransformError>(())
      })
      .unwrap();

    assert!(chunks.len() > 1);

    // Check the chunks are contiguous and reassemble to the original samples
    let mut next_sample_offset = 0u64;
    let mut channel_samples: [Vec<i64>; 2] = [vec![], vec![]];

    for chunk in chunks.iter() {
      assert_eq!(chunk.multiplex_group().as_ref(), &group);
      assert_eq!(chunk.multiplex_group_index(), 0);
      assert_eq!(chunk.sample_offset(), next_sample_offset);

      next_sample_offset += u64::from(chunk.number_of_samples());

      let decoded = chunk.channel_samples().unwrap();
      for (samples, decoded) in channel_samples.iter_mut().zip(decoded) {
        samples.extend(decoded);
      }
    }

    assert_eq!(next_sample_offset, 100);
    assert_eq!(channel_samples[0], samples[0]);
    assert_eq!(channel_samples[1], samples[1]);
  }

  #[test]
  fn errors_when_waveform_data_is_missing() {
    let mut data_set = DataSet::new();
    data_set
      .insert_sequence_value(
        &dictionary::WAVEFORM_SEQUENCE,
        vec![test_multiplex_group(1, 2).to_data_set().unwrap()],
      )
      .unwrap();

    let mut transform = P10WaveformChunkTransform::new();

    assert!(add_tokens_for_data_set(&data_set, &mut transform).is_err());
  }

  #[test]
  fn errors_when_waveform_data_length_is_invalid() {
    let group = test_multiplex_group(1, 3);

    // Two bytes of waveform data is not enough for three 16-bit samples
    let data_set = test_data_set(&[(&group, vec![0, 0])]);

    let mut transform = P10WaveformChunkTransform::new();

    assert!(add_tokens_for_data_set(&data_set, &mut transform).is_err());
  }
}
