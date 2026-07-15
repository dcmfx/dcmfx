//! Encoding of waveform sample data.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::DcmfxError;

use crate::iods::waveform_module::WaveformSampleInterpretation;

/// An error that occurred when encoding waveform sample data.
///
#[derive(Clone, Debug, PartialEq)]
pub enum WaveformEncodeError {
  /// The sample data does not hold a whole number of sample sets, i.e. the
  /// channels of sample data are not all the same length.
  ChannelLengthsUnequal,

  /// A sample value is out of range for the sample interpretation.
  SampleValueOutOfRange {
    channel_index: usize,
    sample_index: usize,
  },
}

impl WaveformEncodeError {
  /// Returns the name of the waveform encode error as a human-readable
  /// string.
  ///
  pub fn name(&self) -> String {
    match self {
      Self::ChannelLengthsUnequal => "Channel lengths unequal".to_string(),
      Self::SampleValueOutOfRange { .. } => {
        "Sample value out of range".to_string()
      }
    }
  }
}

impl core::fmt::Display for WaveformEncodeError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::ChannelLengthsUnequal => write!(
        f,
        "Waveform sample data does not hold a whole number of sample sets"
      ),

      Self::SampleValueOutOfRange {
        channel_index,
        sample_index,
      } => write!(
        f,
        "Waveform sample value at index {sample_index} in channel \
         {channel_index} is out of range for the sample interpretation"
      ),
    }
  }
}

impl DcmfxError for WaveformEncodeError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("Waveform encode error {task_description}"),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match self {
      Self::SampleValueOutOfRange {
        channel_index,
        sample_index,
      } => {
        lines.push(format!("  Channel index: {channel_index}"));
        lines.push(format!("  Sample index: {sample_index}"));
      }

      Self::ChannelLengthsUnequal => (),
    }

    lines
  }
}

/// Appends the little-endian bytes for a raw stored value to a byte vector.
/// The raw stored value must already be in range for the sample
/// interpretation, e.g. by having been validated with
/// [`validate_raw_sample()`].
///
pub(crate) fn encode_raw_sample(
  raw_value: i64,
  sample_interpretation: WaveformSampleInterpretation,
  bytes: &mut Vec<u8>,
) {
  let bytes_per_sample = sample_interpretation.bytes_per_sample();

  bytes.extend_from_slice(&raw_value.to_le_bytes()[0..bytes_per_sample]);
}

/// Encodes a raw stored value to the little-endian bytes for an OB/OW data
/// element that stores a single value in a multiplex group's sample encoding,
/// i.e. the waveform padding value and the channel minimum and maximum
/// values. A padding byte is appended when the sample encoding is a single
/// byte, as binary data elements must have even length.
///
pub(crate) fn encode_stored_value_bytes(
  raw_value: i64,
  sample_interpretation: WaveformSampleInterpretation,
) -> Vec<u8> {
  let mut bytes = Vec::with_capacity(8);

  encode_raw_sample(raw_value, sample_interpretation, &mut bytes);

  if bytes.len() % 2 == 1 {
    bytes.push(0);
  }

  bytes
}

/// Validates that a raw stored value is in range for the given sample
/// interpretation. For the companded sample interpretations values are raw
/// G.711 bytes.
///
pub(crate) fn validate_raw_sample(
  raw_value: i64,
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<i64, ()> {
  use WaveformSampleInterpretation as SI;

  match sample_interpretation {
    SI::SignedByte => {
      in_range(raw_value, i64::from(i8::MIN), i64::from(i8::MAX))
    }
    SI::UnsignedByte | SI::MuLawByte | SI::ALawByte => {
      in_range(raw_value, 0, i64::from(u8::MAX))
    }

    SI::SignedShort => {
      in_range(raw_value, i64::from(i16::MIN), i64::from(i16::MAX))
    }
    SI::UnsignedShort => in_range(raw_value, 0, i64::from(u16::MAX)),

    SI::SignedLong => {
      in_range(raw_value, i64::from(i32::MIN), i64::from(i32::MAX))
    }
    SI::UnsignedLong => in_range(raw_value, 0, i64::from(u32::MAX)),

    SI::SignedVeryLong => Ok(raw_value),
    SI::UnsignedVeryLong => in_range(raw_value, 0, i64::MAX),
  }
}

fn in_range(value: i64, min: i64, max: i64) -> Result<i64, ()> {
  if value >= min && value <= max {
    Ok(value)
  } else {
    Err(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_negative_unsigned_very_long_values() {
    use WaveformSampleInterpretation as SI;

    assert_eq!(validate_raw_sample(-1, SI::UnsignedVeryLong), Err(()));
    assert_eq!(
      validate_raw_sample(i64::MAX, SI::UnsignedVeryLong),
      Ok(i64::MAX)
    );
  }
}
