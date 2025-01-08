use std::ops::Range;
use std::rc::Rc;

/// A single frame of raw pixel data. This is made up of a one or more slices
/// into reference-counted `Vec<u8>` data, which avoids copying of data.
///
/// If required, use [`PixelDataRawFrame::to_bytes()`] to get the raw frame data
/// in a single contiguous buffer.
///
#[derive(Clone, Debug)]
pub struct PixelDataRawFrame {
  fragments: Vec<(Rc<Vec<u8>>, Range<usize>)>,
  length: usize,
}

impl PixelDataRawFrame {
  /// Creates a new empty frame of raw pixel data.
  ///
  pub(crate) fn new() -> Self {
    PixelDataRawFrame {
      fragments: vec![],
      length: 0,
    }
  }

  /// Adds the next fragment of raw pixel data to this frame.
  ///
  pub(crate) fn push_fragment(
    &mut self,
    data: Rc<Vec<u8>>,
    range: Range<usize>,
  ) {
    if range.start > data.len() || range.end > data.len() {
      panic!(
        "Invalid raw pixel data fragment range: {:?}, length: {}",
        range,
        data.len()
      );
    }

    self.length += range.len();
    self.fragments.push((data, range));
  }

  /// The size in bytes of this frame of raw pixel data.
  ///
  pub fn len(&self) -> usize {
    self.length
  }

  /// Returns whether this frame of raw pixel data is empty.
  ///
  pub fn is_empty(&self) -> bool {
    self.length == 0
  }

  /// Returns the fragments of binary data that make up this frame of raw pixel
  /// data.
  ///
  pub fn fragments(&self) -> Vec<&[u8]> {
    self
      .fragments
      .iter()
      .map(|fragment| &fragment.0[fragment.1.clone()])
      .collect()
  }

  /// Removes `count` bytes from the end of this frame of raw pixel data.
  ///
  pub(crate) fn drop_end_bytes(&mut self, count: usize) {
    let target_length = self.length.saturating_sub(count);

    // While this raw frame exceeds the target length, pop off the last fragment
    while self.len() > target_length {
      match self.fragments.pop() {
        Some(fragment) => {
          self.length -= fragment.1.len();

          // If this raw frame is now too short then restore it, but with a
          // reduced range that exactly meets the target length
          if self.length < target_length {
            let fragment_length = target_length - self.length;
            let new_fragment = (
              fragment.0,
              fragment.1.start..(fragment.1.start + fragment_length),
            );

            self.fragments.push(new_fragment);
            self.length = target_length;

            break;
          }
        }

        None => break,
      }
    }
  }

  /// Converts this frame of raw pixel data to a single contiguous `Vec<u8>`.
  /// This may require copying the raw pixel data into a new contiguous buffer,
  /// so accessing the individual fragments is preferred when possible.
  ///
  pub fn to_bytes(&self) -> Rc<Vec<u8>> {
    // If there's a single fragment with all the data then return it and avoid a
    // copy
    if let Some(first) = self.fragments.first() {
      if first.0.len() == self.len() && first.1.start == 0 {
        return first.0.clone();
      }
    }

    // Copy the fragments into a new buffer
    let mut buffer = Vec::with_capacity(self.len());
    for item in self.fragments.iter() {
      buffer.extend_from_slice(&item.0[item.1.clone()]);
    }

    Rc::new(buffer)
  }
}

impl PartialEq for PixelDataRawFrame {
  fn eq(&self, other: &Self) -> bool {
    self.to_bytes() == other.to_bytes()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn single_fragment_test() {
    let mut frame = PixelDataRawFrame::new();

    frame.push_fragment(Rc::new(vec![0, 1, 2, 3]), 0..3);

    assert_eq!(frame.len(), 3);
    assert_eq!(frame.fragments(), vec![&[0, 1, 2]]);
    assert_eq!(frame.to_bytes(), Rc::new(vec![0, 1, 2]));
  }

  #[test]
  fn multiple_fragments_test() {
    let mut frame = PixelDataRawFrame::new();

    frame.push_fragment(Rc::new(vec![0, 1, 2, 3]), 0..2);
    frame.push_fragment(Rc::new(vec![4, 5, 6, 7]), 1..3);
    frame.push_fragment(Rc::new(vec![8, 9, 10, 11]), 2..4);

    assert_eq!(frame.len(), 6);
    assert_eq!(frame.fragments(), vec![&[0, 1], &[5, 6], &[10, 11]]);
    assert_eq!(frame.to_bytes(), Rc::new(vec![0, 1, 5, 6, 10, 11]));
  }

  #[test]
  fn drop_end_bytes_test() {
    let mut frame = PixelDataRawFrame::new();
    frame.push_fragment(Rc::new(vec![0, 1, 2, 3, 4]), 0..5);

    frame.drop_end_bytes(2);
    assert_eq!(frame.to_bytes(), Rc::new(vec![0, 1, 2]));

    let mut frame = PixelDataRawFrame::new();
    frame.push_fragment(Rc::new(vec![0, 0, 1, 1]), 1..3);
    frame.push_fragment(Rc::new(vec![2, 3]), 0..2);

    frame.drop_end_bytes(1);
    assert_eq!(frame.to_bytes(), Rc::new(vec![0, 1, 2]));

    let mut frame = PixelDataRawFrame::new();
    frame.push_fragment(Rc::new(vec![0, 1]), 0..2);
    frame.push_fragment(Rc::new(vec![2, 3]), 0..2);
    frame.push_fragment(Rc::new(vec![4, 5]), 0..2);

    frame.drop_end_bytes(2);
    assert_eq!(frame.to_bytes(), Rc::new(vec![0, 1, 2, 3]));
  }
}
