//! Holds a single frame of pixel data in its raw form. Details of how to
//! read or interpret the data are not a concern of [`PixelDataFrame`].

#[cfg(not(feature = "std"))]
use alloc::{vec, vec::Vec};

use dcmfx_core::RcByteSlice;

/// A single frame of pixel data in its raw form. It is made up of a one or more
/// slices into reference-counted `Vec<u8>` data, which avoids copying of data.
///
/// If required, use [`PixelDataFrame::to_bytes()`] to get the frame's data in a
/// single contiguous buffer.
///
#[derive(Clone, Debug, Default)]
pub struct PixelDataFrame {
  frame_index: Option<usize>,
  chunks: Vec<RcByteSlice>,
  length_in_bits: u64,
  bit_offset: usize,
}

impl PixelDataFrame {
  /// Creates a new empty frame of pixel data.
  ///
  pub fn new() -> Self {
    Self::default()
  }

  /// Creates a new frame of pixel data with the given data.
  ///
  pub fn new_from_bytes(data: Vec<u8>) -> Self {
    let mut frame = Self::default();
    frame.push_bytes(data.into());
    frame
  }

  /// Returns the index of this frame, i.e. 0 for the first frame in its DICOM
  /// data set, 1 for the second frame, etc. Returns `None` if the frame's index
  /// hasn't been set.
  ///
  pub fn index(&self) -> Option<usize> {
    self.frame_index
  }

  /// Sets the index of this frame.
  ///
  pub fn set_index(&mut self, index: usize) {
    self.frame_index = Some(index);
  }

  /// Adds the next chunk of pixel data to this frame.
  ///
  pub fn push_bytes(&mut self, fragment: RcByteSlice) {
    self.length_in_bits += fragment.len() as u64 * 8;
    self.chunks.push(fragment);
  }

  /// Adds the next chunk of pixel data to this frame.
  ///
  pub fn push_bits(&mut self, chunk: RcByteSlice, length_in_bits: u64) {
    self.length_in_bits += length_in_bits;
    self.chunks.push(chunk);
  }

  /// The size in bytes of this frame of pixel data.
  ///
  pub fn len(&self) -> u64 {
    self.len_bits().div_ceil(8)
  }

  /// The size in bits of this frame of pixel data.
  ///
  pub fn len_bits(&self) -> u64 {
    self.length_in_bits.saturating_sub(self.bit_offset as u64)
  }

  /// Returns the bit offset for this frame.
  ///
  /// The bit offset is only relevant to native multi-frame pixel data that has
  /// a *'(0028,0010) Bits Allocated'* value of 1, where it specifies how many
  /// high bits in this frame's first byte should be ignored when reading its
  /// data. In all other cases it is zero and is unused.
  ///
  pub fn bit_offset(&self) -> usize {
    self.bit_offset
  }

  /// Sets this frame's pixel data bit offset. See [`Self::bit_offset()`] for
  /// details.
  ///
  pub fn set_bit_offset(&mut self, bit_offset: usize) {
    self.bit_offset = bit_offset.clamp(0, 7);
  }

  /// Returns whether this frame of pixel data is empty.
  ///
  pub fn is_empty(&self) -> bool {
    self.len_bits() == 0
  }

  /// Returns the chunks of binary data that make up this frame of pixel
  /// data.
  ///
  pub fn chunks(&self) -> &[RcByteSlice] {
    &self.chunks
  }

  /// Removes `count` bytes from the end of this frame of pixel data.
  ///
  pub(crate) fn drop_end_bytes(&mut self, count: usize) {
    let target_length = self.len().saturating_sub(count as u64);

    // While this frame exceeds the target length, pop off the last fragment
    while self.len() > target_length {
      match self.chunks.pop() {
        Some(fragment) => {
          self.length_in_bits -= fragment.len() as u64 * 8;

          // If this frame is now too short then restore it, but with a reduced
          // range that exactly meets the target length
          if self.len() < target_length {
            let fragment_length = target_length - self.len();
            let new_fragment = fragment.take(fragment_length as usize);

            self.chunks.push(new_fragment);
            self.length_in_bits = target_length * 8;

            break;
          }
        }

        None => break,
      }
    }
  }

  /// Converts this frame of pixel data to a single contiguous `Vec<u8>`. This
  /// may require copying the pixel data into a new contiguous buffer, so
  /// accessing the individual chunks is preferred when possible.
  ///
  pub fn to_bytes(&self) -> RcByteSlice {
    // If there's a single chunk then return it and avoid a copy. This isn't
    // possible when there's a non-zero bit offset.
    if self.bit_offset == 0 && self.chunks.len() == 1 {
      return self.chunks[0].clone();
    }

    // Copy the chunks into a new buffer
    let mut buffer = Vec::with_capacity(self.len() as usize);
    for item in self.chunks.iter() {
      buffer.extend_from_slice(item);
    }

    // Correct for any bit offset by left shifting the whole buffer. This is
    // only used by 1bpp pixel data frames that have a pixel count that's not a
    // multiple of eight.
    if self.bit_offset != 0 {
      for i in 0..buffer.len() {
        let next_byte = buffer.get(i + 1).unwrap_or(&0);
        buffer[i] =
          (buffer[i] >> self.bit_offset) | (next_byte << (8 - self.bit_offset));
      }
    }

    RcByteSlice::from_vec(buffer)
  }

  /// If this frame of pixel data contains more than one chunk, combines them
  /// into one new chunk. Returns the slice of the first (and only) chunk that
  /// contains all the pixel data for this frame.
  ///
  pub fn combine_fragments(&mut self) -> &[u8] {
    if self.chunks.is_empty() {
      self.chunks = vec![RcByteSlice::empty()];
    }

    if self.chunks.len() > 1 {
      self.chunks = vec![self.to_bytes()];
    }

    &self.chunks()[0]
  }
}

impl PartialEq for PixelDataFrame {
  fn eq(&self, other: &Self) -> bool {
    self.to_bytes() == other.to_bytes()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_chunk_test() {
    let mut frame = PixelDataFrame::new();

    frame.push_bytes(RcByteSlice::from_vec(vec![0, 1, 2, 3]).take(3));

    assert_eq!(frame.len(), 3);
    assert_eq!(frame.chunks(), vec![vec![0, 1, 2].into()]);
    assert_eq!(frame.to_bytes(), vec![0, 1, 2].into());
  }

  #[test]
  fn multiple_chunks_test() {
    let mut frame = PixelDataFrame::new();

    frame.push_bytes(RcByteSlice::from_vec(vec![0, 1, 2, 3]).take(2));
    frame.push_bytes(RcByteSlice::from_vec(vec![4, 5, 6, 7]).slice(1, 3));
    frame.push_bytes(RcByteSlice::from_vec(vec![8, 9, 10, 11]).drop(2));

    assert_eq!(frame.len(), 6);
    assert_eq!(
      frame.chunks(),
      vec![vec![0, 1].into(), vec![5, 6].into(), vec![10, 11].into()]
    );
    assert_eq!(frame.to_bytes(), vec![0, 1, 5, 6, 10, 11].into());
  }

  #[test]
  fn drop_end_bytes_test() {
    let mut frame = PixelDataFrame::new();
    frame.push_bytes(vec![0, 1, 2, 3, 4].into());

    frame.drop_end_bytes(2);
    assert_eq!(frame.to_bytes(), vec![0, 1, 2].into());

    let mut frame = PixelDataFrame::new();
    frame.push_bytes(RcByteSlice::from_vec(vec![0, 0, 1, 1]).slice(1, 3));
    frame.push_bytes(vec![2, 3].into());

    frame.drop_end_bytes(1);
    assert_eq!(frame.to_bytes(), vec![0, 1, 2].into());

    let mut frame = PixelDataFrame::new();
    frame.push_bytes(vec![0, 1].into());
    frame.push_bytes(vec![2, 3].into());
    frame.push_bytes(vec![4, 5].into());

    frame.drop_end_bytes(2);
    assert_eq!(frame.to_bytes(), vec![0, 1, 2, 3].into());
  }
}
