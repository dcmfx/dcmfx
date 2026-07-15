//! A chunk of the waveform data of a waveform multiplex group.

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;

use dcmfx_core::{Rc, RcByteSlice};

use crate::{WaveformDecodeError, WaveformMultiplexGroup, decode};

/// A chunk of the waveform data of a single waveform multiplex group, holding
/// consecutive whole sample sets in channel-interleaved order, along with the
/// multiplex group the sample sets belong to.
///
/// A chunk's samples are decoded using [`WaveformChunk::channel_samples()`].
///
#[derive(Clone, Debug, PartialEq)]
pub struct WaveformChunk {
  multiplex_group: Rc<WaveformMultiplexGroup>,
  multiplex_group_index: usize,
  sample_offset: u64,
  number_of_samples: u32,
  data: RcByteSlice,
}

impl WaveformChunk {
  /// Creates a new [`WaveformChunk`] with the given raw channel-interleaved
  /// waveform data.
  ///
  pub fn new(
    multiplex_group: Rc<WaveformMultiplexGroup>,
    multiplex_group_index: usize,
    sample_offset: u64,
    number_of_samples: u32,
    data: RcByteSlice,
  ) -> Self {
    Self {
      multiplex_group,
      multiplex_group_index,
      sample_offset,
      number_of_samples,
      data,
    }
  }

  /// Returns the waveform multiplex group that this chunk holds sample sets
  /// for.
  ///
  pub fn multiplex_group(&self) -> &Rc<WaveformMultiplexGroup> {
    &self.multiplex_group
  }

  /// Returns the index of the waveform multiplex group that this chunk holds
  /// sample sets for, i.e. the index of the item in the *'(5400,0100)
  /// Waveform Sequence'*.
  ///
  pub fn multiplex_group_index(&self) -> usize {
    self.multiplex_group_index
  }

  /// Returns the index of this chunk's first sample set within its waveform
  /// multiplex group.
  ///
  pub fn sample_offset(&self) -> u64 {
    self.sample_offset
  }

  /// Returns the number of samples per channel in this chunk.
  ///
  pub fn number_of_samples(&self) -> u32 {
    self.number_of_samples
  }

  /// Returns this chunk's raw waveform data bytes. These hold this chunk's
  /// whole sample sets in channel-interleaved order, i.e. one sample for
  /// every channel at the first instant, followed by one sample for every
  /// channel at the next instant, and so on. Each sample is stored
  /// little-endian in the multiplex group's sample interpretation, making a
  /// sample set [`WaveformMultiplexGroup::number_of_channels()`] ×
  /// [`WaveformSampleInterpretation::bytes_per_sample()`] bytes in size.
  ///
  /// Use [`WaveformChunk::channel_samples()`] to decode this data into
  /// per-channel sample values.
  ///
  /// [`WaveformSampleInterpretation::bytes_per_sample()`]:
  ///   crate::WaveformSampleInterpretation::bytes_per_sample
  ///
  pub fn data(&self) -> &RcByteSlice {
    &self.data
  }

  /// Returns the raw stored values for every channel in this chunk, indexed
  /// by channel. All channels are decoded in a single pass over the chunk's
  /// sample sets. For the companded
  /// [`WaveformSampleInterpretation::MuLawByte`] and
  /// [`WaveformSampleInterpretation::ALawByte`] sample interpretations the
  /// values are raw G.711 bytes, i.e. they are not expanded to linear PCM.
  ///
  /// Samples equal to [`WaveformMultiplexGroup::padding_value()`] are not
  /// real measurements, e.g. they may fill out a channel that has fewer
  /// samples than the rest of its multiplex group.
  ///
  /// [`WaveformSampleInterpretation::MuLawByte`]:
  ///   crate::WaveformSampleInterpretation::MuLawByte
  /// [`WaveformSampleInterpretation::ALawByte`]:
  ///   crate::WaveformSampleInterpretation::ALawByte
  ///
  pub fn channel_samples(&self) -> Result<Vec<Vec<i64>>, WaveformDecodeError> {
    decode::decode_channel_samples(
      &self.data,
      usize::from(self.multiplex_group.number_of_channels()),
      self.number_of_samples as usize,
      self.multiplex_group.sample_interpretation(),
    )
  }
}
