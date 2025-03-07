//! Work with the DICOM `IntegerString` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use crate::DataError;

/// Converts a `IntegerString` value to a list of ints.
///
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<i32>, DataError> {
  let integer_string = core::str::from_utf8(bytes).map_err(|_| {
    DataError::new_value_invalid("IntegerString is invalid UTF-8".to_string())
  })?;

  let integer_string = integer_string.trim_matches('\0');

  integer_string
    .split('\\')
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| s.parse::<i32>())
    .collect::<Result<Vec<i32>, _>>()
    .map_err(|_| {
      DataError::new_value_invalid(format!(
        "IntegerString is invalid: '{}'",
        integer_string
      ))
    })
}

/// Converts a list of ints to an `IntegerString` value.
///
pub fn to_bytes(values: &[i32]) -> Vec<u8> {
  let mut bytes = values
    .iter()
    .map(|f| f.to_string())
    .collect::<Vec<String>>()
    .join("\\")
    .into_bytes();

  if bytes.len() % 2 == 1 {
    bytes.push(0x20);
  }

  bytes
}

#[cfg(test)]
mod tests {
  use super::*;

  #[cfg(not(feature = "std"))]
  use alloc::vec;

  #[test]
  fn from_bytes_test() {
    assert_eq!(from_bytes(&[]), Ok(vec![]));

    assert_eq!(from_bytes(b" "), Ok(vec![]));

    assert_eq!(from_bytes(b" 1"), Ok(vec![1]));

    assert_eq!(from_bytes(b" 1\\2 "), Ok(vec![1, 2]));

    assert_eq!(
      from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "IntegerString is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      from_bytes(b"A"),
      Err(DataError::new_value_invalid(
        "IntegerString is invalid: 'A'".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(to_bytes(&[]), vec![]);

    assert_eq!(to_bytes(&[1]), b"1 ".to_vec());

    assert_eq!(to_bytes(&[1, 2]), b"1\\2 ".to_vec());
  }
}
