//! Work with the DICOM `AttributeTag` value representation.

#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec};

use crate::{DataElementTag, DataError};

/// Converts an `AttributeTag` value into data element tags.
///
pub fn from_bytes(bytes: &[u8]) -> Result<Vec<DataElementTag>, DataError> {
  if bytes.len() % 4 != 0 {
    return Err(DataError::new_value_invalid(
      "AttributeTag data length is not a multiple of 4".to_string(),
    ));
  }

  let mut tags = Vec::<DataElementTag>::new();

  for chunk in bytes.chunks_exact(4) {
    let group = u16::from_le_bytes([chunk[0], chunk[1]]);
    let element = u16::from_le_bytes([chunk[2], chunk[3]]);

    tags.push(DataElementTag::new(group, element));
  }

  Ok(tags)
}

/// Converts data element tags into an `AttributeTag` value.
///
pub fn to_bytes(values: &[DataElementTag]) -> Vec<u8> {
  let mut bytes = Vec::<u8>::with_capacity(values.len() * 4);

  for tag in values {
    bytes.extend_from_slice(&tag.group.to_le_bytes());
    bytes.extend_from_slice(&tag.element.to_le_bytes());
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

    assert_eq!(
      from_bytes(&[0x10, 0x48, 0xFE, 0x00, 0x52, 0x30, 0x41, 0x9A]),
      Ok(vec![
        DataElementTag::new(0x4810, 0x00FE,),
        DataElementTag::new(0x3052, 0x9A41,)
      ])
    );

    assert_eq!(
      from_bytes(&[0x00, 0x01]),
      Err(DataError::new_value_invalid(
        "AttributeTag data length is not a multiple of 4".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(to_bytes(&[]), vec![]);

    assert_eq!(
      to_bytes(&[
        DataElementTag::new(0x4810, 0x00FE),
        DataElementTag::new(0x1234, 0x5678)
      ]),
      vec![0x10, 0x48, 0xFE, 0x00, 0x34, 0x12, 0x78, 0x56]
    );
  }
}
