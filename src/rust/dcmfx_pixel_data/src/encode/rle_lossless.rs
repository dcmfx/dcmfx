#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

/// RLE encodes the data for a single segment where each row of the image data
/// has the given size.
///
/// The row size is needed because each row is RLE encoded separately and then
/// concatenated.
///
/// Ref: PS3.5 G.3.1.
///
#[allow(dead_code)]
pub fn encode_segment(data: &[u8], row_size: usize) -> Result<Vec<u8>, ()> {
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
