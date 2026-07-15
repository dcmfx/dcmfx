//! Decoding of waveform sample data.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec,
  vec::Vec,
};

use dcmfx_core::DcmfxError;

use crate::iods::waveform_module::WaveformSampleInterpretation;

/// An error that occurred when decoding waveform sample data.
///
#[derive(Clone, Debug, PartialEq)]
pub enum WaveformDecodeError {
  /// The waveform data does not have the expected length given the multiplex
  /// group's number of channels, number of samples, and bits allocated.
  DataLengthInvalid { expected: usize, actual: usize },

  /// A sample value stored using the
  /// [`WaveformSampleInterpretation::UnsignedVeryLong`] sample interpretation
  /// exceeds [`i64::MAX`] and so can't be represented.
  SampleValueOverflow,
}

impl WaveformDecodeError {
  /// Returns the name of the waveform decode error as a human-readable
  /// string.
  ///
  pub fn name(&self) -> String {
    match self {
      Self::DataLengthInvalid { .. } => "Data length invalid".to_string(),
      Self::SampleValueOverflow => "Sample value overflow".to_string(),
    }
  }
}

impl core::fmt::Display for WaveformDecodeError {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    match self {
      Self::DataLengthInvalid { expected, actual } => write!(
        f,
        "Waveform data length is invalid, expected {expected} bytes but \
         found {actual} bytes"
      ),

      Self::SampleValueOverflow => write!(
        f,
        "Waveform sample value exceeds the maximum supported value of 2^63 - 1"
      ),
    }
  }
}

impl DcmfxError for WaveformDecodeError {
  fn to_lines(&self, task_description: &str) -> Vec<String> {
    let mut lines = vec![
      format!("Waveform decode error {task_description}"),
      "".to_string(),
      format!("  Error: {}", self.name()),
    ];

    match self {
      Self::DataLengthInvalid { expected, actual } => {
        lines.push(format!("  Expected length in bytes: {expected}"));
        lines.push(format!("  Actual length in bytes: {actual}"));
      }

      Self::SampleValueOverflow => (),
    }

    lines
  }
}

/// Decodes the raw stored values for every channel from channel-interleaved
/// waveform data, indexed by channel. All channels are decoded in a single
/// pass over the sample sets.
///
pub(crate) fn decode_channel_samples(
  waveform_data: &[u8],
  channel_count: usize,
  number_of_samples: usize,
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<Vec<Vec<i64>>, WaveformDecodeError> {
  let bytes_per_sample = sample_interpretation.bytes_per_sample();
  let stride = channel_count * bytes_per_sample;

  let expected_length = number_of_samples.saturating_mul(stride);

  if waveform_data.len() != expected_length {
    return Err(WaveformDecodeError::DataLengthInvalid {
      expected: expected_length,
      actual: waveform_data.len(),
    });
  }

  let mut channels: Vec<Vec<i64>> = (0..channel_count)
    .map(|_| Vec::with_capacity(number_of_samples))
    .collect();

  if channel_count == 0 {
    return Ok(channels);
  }

  // The dispatch on the sample interpretation is hoisted out of the
  // per-sample loop as this is the hot path when decoding large recordings
  use WaveformSampleInterpretation as SI;

  match sample_interpretation {
    SI::SignedByte => {
      decode_sample_sets(waveform_data, &mut channels, 1, |b| {
        Ok(i64::from(b[0] as i8))
      })
    }
    SI::UnsignedByte | SI::MuLawByte | SI::ALawByte => {
      decode_sample_sets(waveform_data, &mut channels, 1, |b| {
        Ok(i64::from(b[0]))
      })
    }

    SI::SignedShort => {
      decode_sample_sets(waveform_data, &mut channels, 2, |b| {
        Ok(i64::from(i16::from_le_bytes([b[0], b[1]])))
      })
    }
    SI::UnsignedShort => {
      decode_sample_sets(waveform_data, &mut channels, 2, |b| {
        Ok(i64::from(u16::from_le_bytes([b[0], b[1]])))
      })
    }

    SI::SignedLong => {
      decode_sample_sets(waveform_data, &mut channels, 4, |b| {
        Ok(i64::from(i32::from_le_bytes([b[0], b[1], b[2], b[3]])))
      })
    }
    SI::UnsignedLong => {
      decode_sample_sets(waveform_data, &mut channels, 4, |b| {
        Ok(i64::from(u32::from_le_bytes([b[0], b[1], b[2], b[3]])))
      })
    }

    SI::SignedVeryLong => {
      decode_sample_sets(waveform_data, &mut channels, 8, |b| {
        Ok(i64::from_le_bytes(b.try_into().unwrap()))
      })
    }
    SI::UnsignedVeryLong => {
      decode_sample_sets(waveform_data, &mut channels, 8, |b| {
        let value = u64::from_le_bytes(b.try_into().unwrap());

        i64::try_from(value)
          .map_err(|_| WaveformDecodeError::SampleValueOverflow)
      })
    }
  }?;

  Ok(channels)
}

/// Decodes channel-interleaved sample sets into per-channel sample values
/// using the given function to decode a single sample.
///
fn decode_sample_sets(
  waveform_data: &[u8],
  channels: &mut [Vec<i64>],
  bytes_per_sample: usize,
  decode: impl Fn(&[u8]) -> Result<i64, WaveformDecodeError>,
) -> Result<(), WaveformDecodeError> {
  let stride = channels.len() * bytes_per_sample;

  for sample_set in waveform_data.chunks_exact(stride) {
    for (channel_index, samples) in channels.iter_mut().enumerate() {
      samples.push(decode(
        &sample_set[channel_index * bytes_per_sample..][..bytes_per_sample],
      )?);
    }
  }

  Ok(())
}

/// Decodes a single raw stored value from little-endian bytes. The length of
/// `bytes` must equal the sample interpretation's number of bytes per sample.
///
pub(crate) fn decode_raw_sample(
  bytes: &[u8],
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<i64, WaveformDecodeError> {
  use WaveformSampleInterpretation as SI;

  let value = match sample_interpretation {
    SI::SignedByte => i64::from(bytes[0] as i8),
    SI::UnsignedByte | SI::MuLawByte | SI::ALawByte => i64::from(bytes[0]),

    SI::SignedShort => i64::from(i16::from_le_bytes([bytes[0], bytes[1]])),
    SI::UnsignedShort => i64::from(u16::from_le_bytes([bytes[0], bytes[1]])),

    SI::SignedLong => {
      i64::from(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    SI::UnsignedLong => {
      i64::from(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    SI::SignedVeryLong => i64::from_le_bytes([
      bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
      bytes[7],
    ]),
    SI::UnsignedVeryLong => {
      let value = u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6],
        bytes[7],
      ]);

      i64::try_from(value)
        .map_err(|_| WaveformDecodeError::SampleValueOverflow)?
    }
  };

  Ok(value)
}

/// Decodes the raw stored value held in an OB/OW data element that stores a
/// single value in a multiplex group's sample encoding, i.e. the waveform
/// padding value and the channel minimum and maximum values.
///
/// A single trailing padding byte is permitted when the sample encoding is a
/// single byte, as binary data elements must have even length.
///
pub(crate) fn decode_stored_value_bytes(
  bytes: &[u8],
  sample_interpretation: WaveformSampleInterpretation,
) -> Result<i64, String> {
  let bytes_per_sample = sample_interpretation.bytes_per_sample();

  if bytes.len() < bytes_per_sample
    || bytes.len() > bytes_per_sample.next_multiple_of(2)
  {
    return Err(format!(
      "Value length of {} bytes is invalid for a sample encoding of {} bytes",
      bytes.len(),
      bytes_per_sample
    ));
  }

  decode_raw_sample(&bytes[0..bytes_per_sample], sample_interpretation).map_err(
    |_| "Value exceeds the maximum supported value of 2^63 - 1".to_string(),
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn decode_raw_sample_for_all_interpretations() {
    use WaveformSampleInterpretation as SI;

    assert_eq!(decode_raw_sample(&[0x80], SI::SignedByte), Ok(-128));
    assert_eq!(decode_raw_sample(&[0x80], SI::UnsignedByte), Ok(128));
    assert_eq!(decode_raw_sample(&[0xFF], SI::MuLawByte), Ok(255));
    assert_eq!(decode_raw_sample(&[0xD5], SI::ALawByte), Ok(213));

    assert_eq!(
      decode_raw_sample(&[0x00, 0x80], SI::SignedShort),
      Ok(-32768)
    );
    assert_eq!(
      decode_raw_sample(&[0x00, 0x80], SI::UnsignedShort),
      Ok(32768)
    );

    assert_eq!(
      decode_raw_sample(&[0x00, 0x00, 0x00, 0x80], SI::SignedLong),
      Ok(i64::from(i32::MIN))
    );
    assert_eq!(
      decode_raw_sample(&[0xFF, 0xFF, 0xFF, 0xFF], SI::UnsignedLong),
      Ok(i64::from(u32::MAX))
    );

    assert_eq!(
      decode_raw_sample(
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80],
        SI::SignedVeryLong
      ),
      Ok(i64::MIN)
    );
    assert_eq!(
      decode_raw_sample(
        &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F],
        SI::UnsignedVeryLong
      ),
      Ok(i64::MAX)
    );
    assert_eq!(
      decode_raw_sample(
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80],
        SI::UnsignedVeryLong
      ),
      Err(WaveformDecodeError::SampleValueOverflow)
    );
  }
}
