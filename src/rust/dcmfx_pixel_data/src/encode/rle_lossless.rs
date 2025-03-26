#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

/// RLE encodes the data for a set of segments where each row of the image data
/// has the given size.
///
/// The returned data includes the RLE Lossless header that specifies the number
/// of segments and their size.
///
/// Ref: PS3.5 G.3.1, PS3.5 G.4, PS3.5 G.5.
///
#[allow(dead_code)]
pub fn encode_segments(
  segments: &[&[u8]],
  row_size: usize,
) -> Result<Vec<u8>, ()> {
  // The maximum number of segments allowed by RLE Lossless is 15
  if segments.len() > 15 {
    return Err(());
  }

  let mut encoded_segments = Vec::with_capacity(segments.len());
  let mut total_segment_length = 0;

  // RLE encode all segments
  for segment in segments {
    let mut data: Vec<u8> = encode_segment(segment, row_size)?;

    // Ensure encoded segment has even length
    if data.len() % 2 == 1 {
      data.push(0)
    }

    total_segment_length += data.len();
    encoded_segments.push(data);
  }

  // Check total output size doesn't exceed a u32
  if 64 + total_segment_length > u32::MAX as usize {
    return Err(());
  }

  let mut output: Vec<u8> = Vec::with_capacity(64 + total_segment_length);

  // Append number of segments
  output.extend_from_slice(&(segments.len() as u32).to_le_bytes());

  // Append segment offsets
  let mut offset = 64;
  for segment in encoded_segments.iter() {
    output.extend_from_slice(&(offset as u32).to_le_bytes());
    offset += segment.len();
  }

  // Pad header to 64 bytes
  output.resize(64, 0);

  // Append encoded segment data
  for segment in encoded_segments.iter() {
    output.extend_from_slice(segment);
  }

  Ok(output)
}

/// RLE encodes the data for a single segment where each row of the image data
/// has the given size.
///
/// The row size is needed because each row is RLE encoded separately and then
/// concatenated.
///
/// Ref: PS3.5 G.3.1.
///
fn encode_segment(data: &[u8], row_size: usize) -> Result<Vec<u8>, ()> {
  let mut output = vec![];

  if row_size == 0 || data.len() % row_size != 0 {
    return Err(());
  }

  for row in data.chunks_exact(row_size) {
    encode_row(row, &mut output)
  }

  Ok(output)
}

/// RLE encodes the data for a single row.
///
fn encode_row(mut data: &[u8], output: &mut Vec<u8>) {
  while !data.is_empty() {
    let mut run_length = 1;

    // See how many times this byte repeats, up to a maximum of 128 which is
    // all the can be encoded in a single run
    while run_length < data.len()
      && data[0] == data[run_length]
      && run_length < 128
    {
      run_length += 1;
    }

    // If there are repeats then encode as a Replicate Run
    if run_length > 1 {
      output.push((257 - run_length) as u8);
      output.push(data[0]);

      data = &data[run_length..];
    }
    // Otherwise encode as a Literal Run
    else {
      let mut length = 1;

      while length < data.len()
        && data[length] != data[length - 1]
        && length < 128
      {
        length += 1;
      }

      // If the last byte in this Literal Run could be part of an immediately
      // following Replicate Run then don't include its final byte
      if length + 1 < data.len() && data[length - 1] == data[length] {
        length -= 1;
      }

      output.push((length - 1) as u8);
      output.extend_from_slice(&data[0..length]);

      data = &data[length..];
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_input() {
    assert_eq!(encode_segment(&[], 1), Ok(vec![]),);
  }

  #[test]
  fn test_single_byte() {
    assert_eq!(encode_segment(&[42], 1), Ok(vec![0, 42]),);
  }

  #[test]
  fn test_all_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], 10),
      Ok(vec![9, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
    );
  }

  #[test]
  fn test_all_repeating_bytes() {
    assert_eq!(encode_segment(&[5, 5, 5, 5], 4), Ok(vec![253, 5]));
  }

  #[test]
  fn test_mixed_repeated_and_unique_bytes() {
    assert_eq!(
      encode_segment(&[1, 2, 3, 4, 5, 5, 5, 6, 7, 8, 8, 9], 12),
      Ok(vec![3, 1, 2, 3, 4, 254, 5, 1, 6, 7, 255, 8, 0, 9]),
    );
  }

  #[test]
  fn test_maximum_rle_run_length() {
    assert_eq!(
      encode_segment(&vec![99; 129], 129),
      Ok(vec![129, 99, 0, 99])
    );
  }
}
