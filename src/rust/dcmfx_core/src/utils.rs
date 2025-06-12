#[cfg(feature = "std")]
use std::rc::Rc;

#[cfg(not(feature = "std"))]
use alloc::{format, rc::Rc, string::String, vec, vec::Vec};

/// A reference counted byte slice holds an `Rc<Vec<u8>>` to a shareable buffer,
/// along with a range that specifies the part of that buffer that this slice
/// refers to.
///
/// This type is used widely to avoid copying buffers wherever possible, and in
/// most cases can be used like a `&[u8]` would be.
///
#[derive(Clone)]
pub struct RcByteSlice {
  data: Rc<Vec<u8>>,
  range: core::ops::Range<usize>,
}

impl RcByteSlice {
  /// Creates a new referenced counted byte slice from a `Vec<u8>`.
  ///
  pub fn from_vec(data: Vec<u8>) -> Self {
    let range = 0..data.len();

    Self {
      data: Rc::new(data),
      range,
    }
  }

  /// Creates an empty reference counted byte slice.
  ///
  pub fn empty() -> Self {
    Self {
      data: Rc::new(vec![]),
      range: 0..0,
    }
  }

  /// Slices this referenced counted byte slice, returning a new reference
  /// counted byte slice that points to the same underlying data.
  ///
  /// This function does not copy any data.
  ///
  pub fn slice(&self, start: usize, end: usize) -> Self {
    assert!(start <= end, "Byte slice range out of bounds");
    assert!(end <= self.range.len(), "Byte slice range out of bounds");

    Self {
      data: self.data.clone(),
      range: (self.range.start + start)..(self.range.start + end),
    }
  }

  /// Returns a new reference counted byte slice with the specified number of
  /// bytes dropped from the front.
  ///
  pub fn drop(&self, n: usize) -> Self {
    self.slice(n, self.len())
  }

  /// Returns a new reference counted byte slice to the specified number of
  /// leading bytes.
  ///
  pub fn take(&self, n: usize) -> Self {
    self.slice(0, n)
  }

  /// Consumes this reference counted byte slice and turns it into a `Vec<u8>`.
  /// Avoids a copy when possible.
  ///
  /// This function copies data if there are multiple references to the
  /// underlying buffer, or its slice bounds do not cover the whole buffer.
  ///
  pub fn into_vec(self) -> Vec<u8> {
    if self.range == (0..self.data.len()) {
      match Rc::try_unwrap(self.data) {
        Ok(data) => data,
        Err(data_rc) => data_rc[self.range.clone()].to_vec(),
      }
    } else {
      self.as_slice().to_vec()
    }
  }

  fn as_slice(&self) -> &[u8] {
    &self.data[self.range.clone()]
  }
}

impl core::fmt::Debug for RcByteSlice {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    write!(f, "{:?}", self.as_slice())
  }
}

impl PartialEq for RcByteSlice {
  fn eq(&self, other: &Self) -> bool {
    self.as_slice() == other.as_slice()
  }
}

impl core::ops::Deref for RcByteSlice {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    self.as_slice()
  }
}

impl From<Vec<u8>> for RcByteSlice {
  fn from(vec: Vec<u8>) -> Self {
    RcByteSlice::from_vec(vec)
  }
}

/// Inspects a byte slice in hexadecimal, e.g. `[1A 2B 3C 4D]`. If the number of
/// bytes in the slice exceeds `max_length` then not all bytes will be
/// shown and a trailing ellipsis will be appended, e.g. `[1A 2B 3C 4D …]`.
///
pub fn inspect_u8_slice(bytes: &[u8], max_length: usize) -> String {
  let byte_count = core::cmp::min(max_length, bytes.len());

  let s = bytes[0..byte_count]
    .iter()
    .map(|byte| format!("{:02X}", byte))
    .collect::<Vec<_>>()
    .join(" ");

  if byte_count == bytes.len() {
    format!("[{}]", s)
  } else {
    format!("[{} …]", s)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::string::ToString;

  #[test]
  fn inspect_u8_slice_test() {
    assert_eq!(
      inspect_u8_slice(&[0xD1, 0x96, 0x33], 100),
      "[D1 96 33]".to_string()
    );

    assert_eq!(
      inspect_u8_slice(&[0xD1, 0x96, 0x33, 0x44], 3),
      "[D1 96 33 …]".to_string()
    );
  }
}
