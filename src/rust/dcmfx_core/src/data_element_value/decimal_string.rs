//! Work with the DICOM `DecimalString` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use crate::DataError;

/// Converts a `DecimalString` value to a list of floats.
///
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<f64>, DataError> {
  let decimal_string = core::str::from_utf8(bytes).map_err(|_| {
    DataError::new_value_invalid("DecimalString is invalid UTF-8".to_string())
  })?;

  let decimal_string = decimal_string.trim_matches('\0');

  decimal_string
    .split('\\')
    .map(|s| s.trim())
    .filter(|s| !s.is_empty())
    .map(|s| s.parse::<f64>())
    .collect::<Result<Vec<f64>, _>>()
    .map_err(|_| {
      DataError::new_value_invalid(format!(
        "DecimalString is invalid: '{}'",
        decimal_string
      ))
    })
}

/// Converts a list of floats to a `DecimalString` value.
///
pub fn to_bytes(values: &[f64]) -> Vec<u8> {
  let values: Vec<String> = values
    .iter()
    .map(|f| {
      let decimal_value = f.to_string();
      let exponential_value = format!("{:e}", f);

      if decimal_value.len() < exponential_value.len() {
        // When exponential notation isn't in use, trim unnecessary zeros
        // and decimal point characters from the end of the string
        let trimmed = if decimal_value.contains('.') {
          decimal_value.trim_end_matches('0').trim_end_matches('.')
        } else {
          &decimal_value
        };

        trimmed[0..core::cmp::min(trimmed.len(), 16)].to_string()
      } else {
        exponential_value
      }
    })
    .collect();

  let mut bytes = values.join("\\").into_bytes();

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

    assert_eq!(from_bytes(b"  1.2"), Ok(vec![1.2]));

    assert_eq!(from_bytes(b"127."), Ok(vec![127.0]));

    assert_eq!(from_bytes(b"-1024"), Ok(vec![-1024.0]));

    assert_eq!(from_bytes(b"  1.2\\4.5"), Ok(vec![1.2, 4.5]));

    assert_eq!(from_bytes(b"1.868344208e-10"), Ok(vec![1.868344208e-10]));

    assert_eq!(from_bytes(b"-0"), Ok(vec![-0.0]));

    assert_eq!(
      from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "DecimalString is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      from_bytes(b"1.A"),
      Err(DataError::new_value_invalid(
        "DecimalString is invalid: '1.A'".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(to_bytes(&[]), vec![]);

    assert_eq!(to_bytes(&[0.0]), b"0 ".to_vec());

    assert_eq!(to_bytes(&[1.2]), b"1.2 ".to_vec());

    assert_eq!(to_bytes(&[1.2, 3.4]), b"1.2\\3.4 ".to_vec());

    assert_eq!(to_bytes(&[1.868344208e-010]), b"1.868344208e-10 ".to_vec());

    assert_eq!(to_bytes(&[1.123456789123456]), b"1.12345678912345".to_vec());
  }
}
