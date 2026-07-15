//! Writes chunks of waveform data as a stream of DICOM P10 tokens.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::{
  DataError, DataSetPath, DcmfxError, RcByteSlice, ValueRepresentation,
  dictionary,
};
use dcmfx_p10::{P10Token, p10_token};

use crate::{WaveformEncodeError, WaveformMultiplexGroup, encode};

/// This writer emits a *'(5400,0100) Waveform Sequence'* as a stream of DICOM
/// P10 tokens, with each multiplex group's sample data provided incrementally
/// in chunks of whole sample sets. This is the writing counterpart to
/// [`crate::transforms::P10WaveformChunkTransform`].
///
/// The emitted tokens are written into a [`dcmfx_p10::P10WriteContext`]
/// alongside the tokens for the rest of the data set. Memory usage is bounded
/// by the size of the sample chunks rather than the size of the waveform
/// data, allowing waveforms of unbounded size, e.g. multi-hour ambulatory ECG
/// recordings, to be written in a stream.
///
/// The number of samples of each multiplex group must be known when the
/// multiplex group begins, because the length of its waveform data is part of
/// its data element header. The waveform data of a single multiplex group
/// can't exceed 2^32 - 2 bytes; recordings larger than this must be split
/// across multiple multiplex groups.
///
pub struct P10WaveformChunkWriter {
  is_sequence_started: bool,
  is_finished: bool,

  // The index of the next item of the Waveform Sequence
  multiplex_group_index: usize,

  // State of the multiplex group currently being written
  current_group: Option<CurrentMultiplexGroup>,
}

struct CurrentMultiplexGroup {
  multiplex_group: WaveformMultiplexGroup,

  // The length in bytes of the waveform data as declared in its data element
  // header, including any trailing padding byte
  padded_length: u64,

  bytes_written: u64,
  samples_written: u64,
}

/// An error that occurred in the process of writing chunks of waveform data
/// as a stream of DICOM P10 tokens.
///
#[derive(Clone, Debug, PartialEq)]
pub enum P10WaveformChunkWriterError {
  /// An error that occurred when serializing the data elements of a multiplex
  /// group.
  DataError(DataError),

  /// An error that occurred when encoding sample data.
  WaveformEncodeError(WaveformEncodeError),

  /// The writer's functions were called in an invalid order.
  WriteOrderInvalid { details: String },

  /// The multiplex group's waveform data exceeds the maximum length of a
  /// data element.
  WaveformDataTooLarge { length: u64 },

  /// The number of samples written to a multiplex group differs from its
  /// declared number of samples.
  SampleCountMismatch { expected: u32, actual: u64 },
}

impl core::fmt::Display for P10WaveformChunkWriterError {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::DataError(e) => e.fmt(f),
      Self::WaveformEncodeError(e) => e.fmt(f),

      Self::WriteOrderInvalid { details } => {
        write!(
          f,
          "Waveform write functions called in an invalid order: {details}"
        )
      }

      Self::WaveformDataTooLarge { length } => write!(
        f,
        "Waveform data length of {length} bytes exceeds the maximum data \
         element length of 2^32 - 2 bytes"
      ),

      Self::SampleCountMismatch { expected, actual } => write!(
        f,
        "Number of waveform samples written is invalid, expected {expected} \
         samples but {actual} samples were written"
      ),
    }
  }
}

impl DcmfxError for P10WaveformChunkWriterError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    match self {
      Self::DataError(e) => e.to_lines(task_description),
      Self::WaveformEncodeError(e) => e.to_lines(task_description),

      Self::WriteOrderInvalid { details } => vec![
        format!("Waveform write error {task_description}"),
        "".to_string(),
        "  Error: Write functions called in an invalid order".to_string(),
        format!("  Details: {details}"),
      ],

      Self::WaveformDataTooLarge { length } => vec![
        format!("Waveform write error {task_description}"),
        "".to_string(),
        "  Error: Waveform data too large".to_string(),
        format!("  Length in bytes: {length}"),
      ],

      Self::SampleCountMismatch { expected, actual } => vec![
        format!("Waveform write error {task_description}"),
        "".to_string(),
        "  Error: Sample count mismatch".to_string(),
        format!("  Expected sample count: {expected}"),
        format!("  Actual sample count: {actual}"),
      ],
    }
  }
}

impl P10WaveformChunkWriter {
  /// Creates a new P10 waveform chunk writer for writing waveform data as a
  /// stream of DICOM P10 tokens.
  ///
  pub fn new() -> Self {
    Self {
      is_sequence_started: false,
      is_finished: false,
      multiplex_group_index: 0,
      current_group: None,
    }
  }

  /// Begins writing the given waveform multiplex group and returns the DICOM
  /// P10 tokens for all of its data elements up to the start of its
  /// *'(5400,1010) Waveform Data'*. The multiplex group's number of samples
  /// specifies how many samples per channel will be written to it.
  ///
  /// The multiplex group's sample data is then provided in chunks via
  /// [`P10WaveformChunkWriter::write_interleaved_samples()`].
  ///
  pub fn begin_multiplex_group(
    &mut self,
    multiplex_group: WaveformMultiplexGroup,
  ) -> Result<Vec<P10Token>, P10WaveformChunkWriterError> {
    if self.is_finished || self.current_group.is_some() {
      return Err(P10WaveformChunkWriterError::WriteOrderInvalid {
        details: "A multiplex group can't begin while the writer is finished \
                  or another multiplex group is being written"
          .to_string(),
      });
    }

    // Check the length of the waveform data fits in its data element header,
    // allowing for a trailing padding byte as binary data elements must have
    // even length
    let padded_length =
      multiplex_group.waveform_data_length().next_multiple_of(2);

    if padded_length > 0xFFFFFFFE {
      return Err(P10WaveformChunkWriterError::WaveformDataTooLarge {
        length: padded_length,
      });
    }

    let mut tokens = vec![];

    // Start the Waveform Sequence when the first multiplex group begins
    if !self.is_sequence_started {
      tokens.push(P10Token::SequenceStart {
        tag: dictionary::WAVEFORM_SEQUENCE.tag,
        vr: ValueRepresentation::Sequence,
        path: DataSetPath::new_with_data_element(
          dictionary::WAVEFORM_SEQUENCE.tag,
        ),
      });

      self.is_sequence_started = true;
    }

    tokens.push(P10Token::SequenceItemStart {
      index: self.multiplex_group_index,
    });

    // Serialize the multiplex group's data elements
    let item_data_set = multiplex_group
      .to_data_set()
      .map_err(P10WaveformChunkWriterError::DataError)?;

    let mut item_path =
      DataSetPath::new_with_data_element(dictionary::WAVEFORM_SEQUENCE.tag);
    item_path
      .add_sequence_item(self.multiplex_group_index)
      .unwrap();

    p10_token::data_elements_to_tokens::<()>(
      &item_data_set,
      &item_path,
      &mut |token| {
        tokens.push(token);
        Ok(())
      },
    )
    .unwrap();

    // Emit the header for the waveform data. Its value bytes follow in the
    // tokens returned by write_interleaved_samples().
    let mut waveform_data_path = item_path.clone();
    waveform_data_path
      .add_data_element(dictionary::WAVEFORM_DATA.tag)
      .unwrap();

    tokens.push(P10Token::DataElementHeader {
      tag: dictionary::WAVEFORM_DATA.tag,
      vr: multiplex_group
        .sample_interpretation()
        .binary_value_representation(),
      length: padded_length as u32,
      path: waveform_data_path,
    });

    self.current_group = Some(CurrentMultiplexGroup {
      multiplex_group,
      padded_length,
      bytes_written: 0,
      samples_written: 0,
    });

    Ok(tokens)
  }

  /// Writes the next chunk of sample data for the current multiplex group and
  /// returns the DICOM P10 tokens for the encoded sample data. The raw sample
  /// values are given as a slice of whole sample sets in channel-interleaved
  /// order, i.e. the order in which they are stored in the waveform data.
  /// Ref: PS3.3 C.10.9.1.5.
  ///
  pub fn write_interleaved_samples(
    &mut self,
    samples: &[i64],
  ) -> Result<Vec<P10Token>, P10WaveformChunkWriterError> {
    let Some(current_group) = self.current_group.as_mut() else {
      return Err(P10WaveformChunkWriterError::WriteOrderInvalid {
        details: "Samples can only be written while a multiplex group is \
                  being written"
          .to_string(),
      });
    };

    let multiplex_group = &current_group.multiplex_group;
    let number_of_channels = usize::from(multiplex_group.number_of_channels());
    let sample_interpretation = multiplex_group.sample_interpretation();

    if number_of_channels == 0 {
      if samples.is_empty() {
        return Ok(vec![]);
      }

      return Err(P10WaveformChunkWriterError::SampleCountMismatch {
        expected: multiplex_group.number_of_samples(),
        actual: samples.len() as u64,
      });
    }

    // The samples must form whole sample sets, i.e. every channel must have
    // the same number of samples
    if !samples.len().is_multiple_of(number_of_channels) {
      return Err(P10WaveformChunkWriterError::WaveformEncodeError(
        WaveformEncodeError::ChannelLengthsUnequal,
      ));
    }

    let samples_per_channel = samples.len() / number_of_channels;

    let samples_written =
      current_group.samples_written + samples_per_channel as u64;

    if samples_written > u64::from(multiplex_group.number_of_samples()) {
      return Err(P10WaveformChunkWriterError::SampleCountMismatch {
        expected: multiplex_group.number_of_samples(),
        actual: samples_written,
      });
    }

    if samples_per_channel == 0 {
      return Ok(vec![]);
    }

    // The samples are already in the waveform data's storage order
    let mut bytes = Vec::with_capacity(
      samples.len() * sample_interpretation.bytes_per_sample() + 1,
    );

    for (index, value) in samples.iter().enumerate() {
      let raw_value =
        encode::validate_raw_sample(*value, sample_interpretation).map_err(
          |_| {
            P10WaveformChunkWriterError::WaveformEncodeError(
              WaveformEncodeError::SampleValueOutOfRange {
                channel_index: index % number_of_channels,
                sample_index: index / number_of_channels,
              },
            )
          },
        )?;

      encode::encode_raw_sample(raw_value, sample_interpretation, &mut bytes);
    }

    current_group.samples_written = samples_written;

    // Append the trailing padding byte on the final bytes of waveform data
    // that has odd length
    if samples_written == u64::from(multiplex_group.number_of_samples())
      && current_group.bytes_written + bytes.len() as u64 + 1
        == current_group.padded_length
    {
      bytes.push(0);
    }

    current_group.bytes_written += bytes.len() as u64;

    let bytes_remaining =
      (current_group.padded_length - current_group.bytes_written) as u32;

    Ok(vec![P10Token::DataElementValueBytes {
      tag: dictionary::WAVEFORM_DATA.tag,
      vr: sample_interpretation.binary_value_representation(),
      data: bytes.into(),
      bytes_remaining,
    }])
  }

  /// Ends the current multiplex group and returns the DICOM P10 tokens that
  /// close it. The number of samples written to the multiplex group must
  /// match its declared number of samples.
  ///
  pub fn end_multiplex_group(
    &mut self,
  ) -> Result<Vec<P10Token>, P10WaveformChunkWriterError> {
    let Some(current_group) = self.current_group.take() else {
      return Err(P10WaveformChunkWriterError::WriteOrderInvalid {
        details: "A multiplex group can only end while one is being written"
          .to_string(),
      });
    };

    if current_group.samples_written
      != u64::from(current_group.multiplex_group.number_of_samples())
    {
      return Err(P10WaveformChunkWriterError::SampleCountMismatch {
        expected: current_group.multiplex_group.number_of_samples(),
        actual: current_group.samples_written,
      });
    }

    let mut tokens = vec![];

    // A multiplex group with no waveform data bytes still emits a final empty
    // token for its waveform data value
    if current_group.padded_length == 0 {
      tokens.push(P10Token::DataElementValueBytes {
        tag: dictionary::WAVEFORM_DATA.tag,
        vr: current_group
          .multiplex_group
          .sample_interpretation()
          .binary_value_representation(),
        data: RcByteSlice::empty(),
        bytes_remaining: 0,
      });
    }

    tokens.push(P10Token::SequenceItemDelimiter);

    self.multiplex_group_index += 1;

    Ok(tokens)
  }

  /// Finishes the Waveform Sequence and returns the DICOM P10 tokens that
  /// close it. No further multiplex groups can be written.
  ///
  /// At least one multiplex group must have been written, as the *'(5400,0100)
  /// Waveform Sequence'* is required to have at least one item.
  ///
  pub fn finish(
    &mut self,
  ) -> Result<Vec<P10Token>, P10WaveformChunkWriterError> {
    if self.is_finished || self.current_group.is_some() {
      return Err(P10WaveformChunkWriterError::WriteOrderInvalid {
        details: "The writer can only finish once all multiplex groups have \
                  ended"
          .to_string(),
      });
    }

    // The Waveform Sequence must have at least one item. Ref: PS3.3 C.10.9.
    if !self.is_sequence_started {
      return Err(P10WaveformChunkWriterError::WriteOrderInvalid {
        details: "The writer can't finish before a multiplex group has been \
                  written because the Waveform Sequence must have at least \
                  one item"
          .to_string(),
      });
    }

    let tokens = vec![P10Token::SequenceDelimiter {
      tag: dictionary::WAVEFORM_SEQUENCE.tag,
    }];

    self.is_finished = true;

    Ok(tokens)
  }
}

impl Default for P10WaveformChunkWriter {
  fn default() -> Self {
    Self::new()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  use dcmfx_p10::DataSetBuilder;

  use crate::transforms::P10WaveformChunkTransform;
  use crate::{
    ChannelDefinition, CodedConcept, DataSetWaveformExtensions,
    WaveformBitsAllocated, WaveformModule, WaveformOriginality,
    WaveformSampleInterpretation,
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
    bits_allocated: WaveformBitsAllocated,
    sample_interpretation: WaveformSampleInterpretation,
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
      bits_allocated,
      sample_interpretation,
      None,
    )
    .unwrap()
  }

  /// Writes two multiplex groups, with the first one's samples split across
  /// two chunks, and returns the emitted tokens along with the multiplex
  /// groups that were written.
  ///
  fn write_test_waveform_tokens() -> (Vec<P10Token>, [WaveformMultiplexGroup; 2])
  {
    let group_0 = test_multiplex_group(
      2,
      5,
      WaveformBitsAllocated::Sixteen,
      WaveformSampleInterpretation::SignedShort,
    );
    let group_1 = test_multiplex_group(
      1,
      2,
      WaveformBitsAllocated::Sixteen,
      WaveformSampleInterpretation::SignedShort,
    );

    let mut writer = P10WaveformChunkWriter::new();
    let mut tokens = vec![];

    tokens.extend(writer.begin_multiplex_group(group_0.clone()).unwrap());
    tokens.extend(
      writer
        .write_interleaved_samples(&[1, 6, 2, 7, 3, 8])
        .unwrap(),
    );
    tokens.extend(writer.write_interleaved_samples(&[4, 9, 5, 10]).unwrap());
    tokens.extend(writer.end_multiplex_group().unwrap());

    tokens.extend(writer.begin_multiplex_group(group_1.clone()).unwrap());
    tokens.extend(writer.write_interleaved_samples(&[-1, -2]).unwrap());
    tokens.extend(writer.end_multiplex_group().unwrap());

    tokens.extend(writer.finish().unwrap());

    (tokens, [group_0, group_1])
  }

  #[test]
  fn written_tokens_build_into_expected_data_set() {
    let (tokens, multiplex_groups) = write_test_waveform_tokens();

    let mut data_set_builder = DataSetBuilder::new();
    for token in tokens.iter() {
      data_set_builder.add_token(token).unwrap();
    }

    data_set_builder.force_end();
    let data_set = data_set_builder.final_data_set().unwrap();

    assert_eq!(
      data_set.waveform_module(),
      Ok(WaveformModule::new(multiplex_groups.to_vec()))
    );
  }

  #[test]
  fn written_tokens_stream_through_chunk_transform() {
    let (tokens, multiplex_groups) = write_test_waveform_tokens();

    let mut transform = P10WaveformChunkTransform::new();

    let mut chunks = vec![];
    for token in tokens.iter() {
      chunks.extend(transform.add_token(token).unwrap());
    }

    // The first multiplex group's samples were written in two chunks
    assert_eq!(chunks.len(), 3);

    assert_eq!(chunks[0].multiplex_group().as_ref(), &multiplex_groups[0]);
    assert_eq!(chunks[0].multiplex_group_index(), 0);
    assert_eq!(chunks[0].sample_offset(), 0);
    assert_eq!(
      chunks[0].channel_samples(),
      Ok(vec![vec![1, 2, 3], vec![6, 7, 8]])
    );

    assert_eq!(chunks[1].multiplex_group_index(), 0);
    assert_eq!(chunks[1].sample_offset(), 3);
    assert_eq!(
      chunks[1].channel_samples(),
      Ok(vec![vec![4, 5], vec![9, 10]])
    );

    assert_eq!(chunks[2].multiplex_group().as_ref(), &multiplex_groups[1]);
    assert_eq!(chunks[2].multiplex_group_index(), 1);
    assert_eq!(chunks[2].channel_samples(), Ok(vec![vec![-1, -2]]));
  }

  #[test]
  fn write_read_round_trips_for_all_interpretations() {
    use WaveformSampleInterpretation as SI;

    let cases: [(WaveformBitsAllocated, SI, [Vec<i64>; 2]); 10] = [
      (
        WaveformBitsAllocated::Eight,
        SI::SignedByte,
        [vec![-128, 0, 127], vec![1, -2, 3]],
      ),
      (
        WaveformBitsAllocated::Eight,
        SI::UnsignedByte,
        [vec![0, 128, 255], vec![1, 2, 3]],
      ),
      (
        WaveformBitsAllocated::Eight,
        SI::MuLawByte,
        [vec![0, 128, 255], vec![1, 2, 3]],
      ),
      (
        WaveformBitsAllocated::Eight,
        SI::ALawByte,
        [vec![0, 128, 255], vec![1, 2, 3]],
      ),
      (
        WaveformBitsAllocated::Sixteen,
        SI::SignedShort,
        [vec![-32768, 0, 32767], vec![80, 90, -85]],
      ),
      (
        WaveformBitsAllocated::Sixteen,
        SI::UnsignedShort,
        [vec![0, 32768, 65535], vec![1, 2, 3]],
      ),
      (
        WaveformBitsAllocated::ThirtyTwo,
        SI::SignedLong,
        [
          vec![i64::from(i32::MIN), 0, i64::from(i32::MAX)],
          vec![1, -2, 3],
        ],
      ),
      (
        WaveformBitsAllocated::ThirtyTwo,
        SI::UnsignedLong,
        [vec![0, 1 << 31, i64::from(u32::MAX)], vec![1, 2, 3]],
      ),
      (
        WaveformBitsAllocated::SixtyFour,
        SI::SignedVeryLong,
        [vec![i64::MIN, 0, i64::MAX], vec![1, -2, 3]],
      ),
      (
        WaveformBitsAllocated::SixtyFour,
        SI::UnsignedVeryLong,
        [vec![0, 1 << 62, i64::MAX], vec![1, 2, 3]],
      ),
    ];

    for (bits_allocated, sample_interpretation, samples) in cases {
      let group = test_multiplex_group(
        samples.len(),
        samples[0].len() as u32,
        bits_allocated,
        sample_interpretation,
      );

      // Interleave the per-channel samples into sample set order
      let mut interleaved = vec![];
      for sample_index in 0..samples[0].len() {
        for channel_samples in samples.iter() {
          interleaved.push(channel_samples[sample_index]);
        }
      }

      // Write the samples and stream the resulting tokens back through a
      // chunk transform
      let mut writer = P10WaveformChunkWriter::new();
      let mut tokens = vec![];
      tokens.extend(writer.begin_multiplex_group(group.clone()).unwrap());
      tokens.extend(writer.write_interleaved_samples(&interleaved).unwrap());
      tokens.extend(writer.end_multiplex_group().unwrap());
      tokens.extend(writer.finish().unwrap());

      let mut transform = P10WaveformChunkTransform::new();
      let mut chunks = vec![];
      for token in tokens.iter() {
        chunks.extend(transform.add_token(token).unwrap());
      }

      assert_eq!(chunks.len(), 1);

      assert_eq!(
        chunks[0].channel_samples(),
        Ok(samples.to_vec()),
        "Round trip failed for {sample_interpretation}",
      );
    }
  }

  #[test]
  fn write_interleaved_samples_errors_on_partial_sample_set() {
    let mut writer = P10WaveformChunkWriter::new();

    writer
      .begin_multiplex_group(test_multiplex_group(
        2,
        5,
        WaveformBitsAllocated::Sixteen,
        WaveformSampleInterpretation::SignedShort,
      ))
      .unwrap();

    // Three values don't form whole sample sets for two channels
    assert_eq!(
      writer.write_interleaved_samples(&[1, 2, 3]),
      Err(P10WaveformChunkWriterError::WaveformEncodeError(
        WaveformEncodeError::ChannelLengthsUnequal,
      ))
    );

    // An out of range value reports its channel and sample indices
    assert_eq!(
      writer.write_interleaved_samples(&[1, 2, 3, 40000]),
      Err(P10WaveformChunkWriterError::WaveformEncodeError(
        WaveformEncodeError::SampleValueOutOfRange {
          channel_index: 1,
          sample_index: 1,
        },
      ))
    );
  }

  #[test]
  fn errors_when_sample_counts_are_invalid() {
    let mut writer = P10WaveformChunkWriter::new();

    writer
      .begin_multiplex_group(test_multiplex_group(
        1,
        5,
        WaveformBitsAllocated::Sixteen,
        WaveformSampleInterpretation::SignedShort,
      ))
      .unwrap();

    // Writing more samples than declared is an error
    assert_eq!(
      writer.write_interleaved_samples(&[1, 2, 3, 4, 5, 6]),
      Err(P10WaveformChunkWriterError::SampleCountMismatch {
        expected: 5,
        actual: 6,
      })
    );

    // Ending the multiplex group with fewer samples than declared is an error
    writer.write_interleaved_samples(&[1, 2, 3]).unwrap();
    assert_eq!(
      writer.end_multiplex_group(),
      Err(P10WaveformChunkWriterError::SampleCountMismatch {
        expected: 5,
        actual: 3,
      })
    );
  }

  #[test]
  fn errors_when_write_order_is_invalid() {
    let mut writer = P10WaveformChunkWriter::new();

    assert!(matches!(
      writer.write_interleaved_samples(&[1]),
      Err(P10WaveformChunkWriterError::WriteOrderInvalid { .. })
    ));

    assert!(matches!(
      writer.end_multiplex_group(),
      Err(P10WaveformChunkWriterError::WriteOrderInvalid { .. })
    ));

    // Finishing before any multiplex groups have been written is an error, as
    // an empty Waveform Sequence is not permitted
    assert!(matches!(
      writer.finish(),
      Err(P10WaveformChunkWriterError::WriteOrderInvalid { .. })
    ));
  }
}
