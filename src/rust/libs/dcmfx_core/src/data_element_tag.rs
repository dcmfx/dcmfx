//! A DICOM data element tag, defined as 16-bit `group` and `element` values.

/// A data element tag that is defined by `group` and `element` values, each of
/// which is a 16-bit unsigned integer.
///
#[derive(Copy, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DataElementTag {
  pub group: u16,
  pub element: u16,
}

impl std::fmt::Display for DataElementTag {
  /// Formats a data element tag as `"($GROUP,$ELEMENT)"`, e.g.`"(0008,0020)"`.
  ///
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    let hex_digits = self.to_hex_digits();

    write!(
      f,
      "({},{})",
      unsafe { std::str::from_utf8_unchecked(&hex_digits[0..4]) },
      unsafe { std::str::from_utf8_unchecked(&hex_digits[4..8]) }
    )
  }
}

impl DataElementTag {
  /// Creates a new data element tag with the given group and element values.
  ///
  pub fn new(group: u16, element: u16) -> Self {
    Self { group, element }
  }

  /// Returns whether the tag is private, which is determined by the group
  /// number being odd.
  ///
  pub fn is_private(&self) -> bool {
    self.group & 1 == 1
  }

  /// Returns whether the tag is for a private creator, which is determined by
  /// the group number being odd and the element being between 0x10 and 0xFF.
  /// Ref: PS3.5 7.8.1.
  ///
  pub fn is_private_creator(&self) -> bool {
    self.is_private() && (0x10..=0xFF).contains(&self.element)
  }

  /// Converts a tag to a single 32-bit integer where the group is in the high
  /// 16 bits and the element is in the low 16 bits.
  ///
  pub fn to_int(&self) -> u32 {
    ((self.group as u32) << 16) | self.element as u32
  }

  /// Formats a data element tag as `"$GROUP$ELEMENT"`, e.g.`"0008002D"`.
  ///
  pub fn to_hex_string(&self) -> String {
    unsafe { std::str::from_utf8_unchecked(&self.to_hex_digits()) }.to_string()
  }

  /// Returns the eight hexadecimal digits for this data element tag's group
  /// and element values.
  ///
  pub fn to_hex_digits(&self) -> [u8; 8] {
    static HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

    [
      HEX_DIGITS[(self.group >> 12) as usize],
      HEX_DIGITS[(self.group >> 8) as usize & 0xF],
      HEX_DIGITS[(self.group >> 4) as usize & 0xF],
      HEX_DIGITS[(self.group) as usize & 0xF],
      HEX_DIGITS[(self.element >> 12) as usize],
      HEX_DIGITS[(self.element >> 8) as usize & 0xF],
      HEX_DIGITS[(self.element >> 4) as usize & 0xF],
      HEX_DIGITS[(self.element) as usize & 0xF],
    ]
  }

  /// Creates a data element tag from a hex string formatted as
  /// `"$GROUP$ELEMENT"`, e.g.`"0008002D"`.
  ///
  #[allow(clippy::result_unit_err)]
  pub fn from_hex_string(tag: &str) -> Result<Self, ()> {
    if tag.len() != 8 {
      return Err(());
    }

    let group = u16::from_str_radix(&tag[0..4], 16).map_err(|_| ())?;
    let element = u16::from_str_radix(&tag[4..8], 16).map_err(|_| ())?;

    Ok(Self { group, element })
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn is_private_test() {
    assert!(DataElementTag::new(0x0001, 0).is_private());

    assert!(!DataElementTag::new(0x0002, 1).is_private());
  }

  #[test]
  fn is_private_creator_test() {
    assert!(DataElementTag::new(0x0001, 0x0010).is_private_creator());

    assert!(DataElementTag::new(0x0001, 0x00FF).is_private_creator());

    assert!(!DataElementTag::new(0x0001, 0x000F).is_private_creator());
  }

  #[test]
  fn to_int_test() {
    assert_eq!(DataElementTag::new(0x1122, 0x3344).to_int(), 0x11223344);
  }

  #[test]
  fn to_string_test() {
    assert_eq!(
      DataElementTag::new(0x1122, 0x3344).to_string(),
      "(1122,3344)"
    );
  }

  #[test]
  fn to_hex_digits_test() {
    assert_eq!(
      DataElementTag::new(0x1122, 0xAABB).to_hex_digits(),
      "1122AABB".as_bytes()
    );
  }

  #[test]
  fn from_hex_string_test() {
    assert_eq!(
      DataElementTag::from_hex_string("11223344"),
      Ok(DataElementTag::new(0x1122, 0x3344))
    );

    assert_eq!(DataElementTag::from_hex_string("1122334"), Err(()));
  }
}
