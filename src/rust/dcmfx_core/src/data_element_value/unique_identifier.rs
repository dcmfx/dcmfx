//! Work with the DICOM `UniqueIdentifier` value representation.

use rand::Rng;
use regex::Regex;

use crate::DataError;

/// Converts a list of UIDs into a `UniqueIdentifier` value.
///
pub fn to_bytes(uids: &[&str]) -> Result<Vec<u8>, DataError> {
  if uids.iter().any(|uid| !is_valid(uid)) {
    return Err(DataError::new_value_invalid(
      "UniqueIdentifier is invalid".to_string(),
    ));
  }

  let mut bytes = uids.join("\\").into_bytes();

  if bytes.len() % 2 == 1 {
    bytes.push(0x00);
  }

  Ok(bytes)
}

static PARSE_UID_REGEX: std::sync::LazyLock<Regex> =
  std::sync::LazyLock::new(|| {
    Regex::new("^(0|[1-9][0-9]*)(\\.(0|[1-9][0-9]*))*$").unwrap()
  });

/// Returns whether the given string is a valid `UniqueIdentifier`. Valid UIDs
/// are 1-64 characters in length, and are made up of sequences of digits
/// separated by the period character. Leading zeros are not permitted in a
/// digit sequence unless the zero is the only digit in the sequence.
///
pub fn is_valid(uid: &str) -> bool {
  // Check the length is valid
  if uid.is_empty() || uid.len() > 64 {
    return false;
  }

  PARSE_UID_REGEX.is_match(uid)
}

/// Generates a new random UID with the given prefix. The new UID will have a
/// length of 64 characters. If a prefix is specified then it must itself be
/// a valid UID and no longer than 60 characters.
///
#[allow(clippy::result_unit_err)]
pub fn new(prefix: &str) -> Result<String, ()> {
  let prefix_length = prefix.len();

  // Check the prefix is valid
  if prefix_length > 60 || !prefix.is_empty() && !is_valid(prefix) {
    return Err(());
  }

  let mut rng = rand::rng();
  let mut random_character = |offset: u32, range: u32| -> char {
    char::from_u32(rng.random_range(offset..(offset + range))).unwrap()
  };

  // Start with a separator, if needed, and a non-zero character
  let mut uid = prefix.to_string();
  if !uid.is_empty() {
    uid.push('.')
  }
  uid.push(random_character(49, 9));

  while uid.len() < 64 {
    uid.push(random_character(48, 10));
  }

  Ok(uid)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_bytes_test() {
    let invalid_uid_error = Err(DataError::new_value_invalid(
      "UniqueIdentifier is invalid".to_string(),
    ));

    assert_eq!(to_bytes(&[]), Ok(vec![]));

    assert_eq!(to_bytes(&[""]), invalid_uid_error);

    assert_eq!(to_bytes(&["1.0"]), Ok(b"1.0\0".to_vec()));

    assert_eq!(to_bytes(&["1.2", "3.4"]), Ok(b"1.2\\3.4\0".to_vec()));

    assert_eq!(to_bytes(&["1.00"]), invalid_uid_error);

    assert_eq!(
      to_bytes(&["1".to_string().repeat(65).as_str()]),
      invalid_uid_error
    );
  }

  #[test]
  fn new_test() {
    for _ in 0..1000 {
      assert!(is_valid(&new("").unwrap()));
      assert!(is_valid(&new("1111.2222").unwrap()));
    }

    assert!(is_valid(&new(("1".repeat(60)).as_str()).unwrap()));

    let uid = new("1111.2222").unwrap();
    assert!(uid.starts_with("1111.2222."));
    assert_eq!(uid.len(), 64);

    assert_eq!(new(("1".repeat(61)).as_str()), Err(()));

    assert_eq!(new("1."), Err(()));
  }
}
