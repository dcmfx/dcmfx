//! A DICOM data element tag, defined as 16-bit `group` and `element` values.

#[cfg(not(feature = "std"))]
use alloc::string::{String, ToString};

/// A data element tag that is defined by `group` and `element` values, each of
/// which is a 16-bit unsigned integer.
///
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DataElementTag {
  pub group: u16,
  pub element: u16,
}

impl core::fmt::Debug for DataElementTag {
  /// Print a tag's group and element in hex when debug printing.
  ///
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let hex_digits = self.to_hex_digits();

    let group = core::str::from_utf8(&hex_digits[0..4]).unwrap();
    let element = core::str::from_utf8(&hex_digits[4..8]).unwrap();

    f.debug_struct("DataElementTag")
      .field("group", &format_args!("0x{group}"))
      .field("element", &format_args!("0x{element}"))
      .finish()
  }
}

impl core::fmt::Display for DataElementTag {
  /// Formats a data element tag as `"($GROUP,$ELEMENT)"`, e.g.`"(0008,0020)"`.
  ///
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    let hex_digits = self.to_hex_digits();

    let group = core::str::from_utf8(&hex_digits[0..4]).unwrap();
    let element = core::str::from_utf8(&hex_digits[4..8]).unwrap();

    write!(f, "({group},{element})")
  }
}

impl DataElementTag {
  /// Creates a new data element tag with the given group and element values.
  ///
  pub fn new(group: u16, element: u16) -> Self {
    Self { group, element }
  }

  /// A data element tag with a group and element set to zero.
  ///
  pub const ZERO: DataElementTag = DataElementTag {
    group: 0,
    element: 0,
  };

  /// Returns whether the tag is File Meta Information, which is determined by
  /// its group number equalling 2.
  ///
  pub fn is_file_meta_information(&self) -> bool {
    self.group == 2
  }

  /// Returns whether the tag is private, which is determined by its group
  /// number being odd.
  ///
  pub fn is_private(&self) -> bool {
    self.group & 1 == 1
  }

  /// Returns whether the tag is for a private creator, which is determined by
  /// its group number being odd and its element being between 0x10 and 0xFF.
  ///
  /// Ref: PS3.5 7.8.1.
  ///
  pub fn is_private_creator(&self) -> bool {
    self.is_private() && (0x10..=0xFF).contains(&self.element)
  }

  /// Returns a copy of this tag with an updated group value.
  ///
  pub fn with_group(&self, group: u16) -> Self {
    Self {
      group,
      element: self.element,
    }
  }

  /// Returns a copy of this tag with an updated element value.
  ///
  pub fn with_element(&self, element: u16) -> Self {
    Self {
      group: self.group,
      element,
    }
  }

  /// Converts a tag to a single 32-bit integer where the group is in the high
  /// 16 bits and the element is in the low 16 bits.
  ///
  pub fn to_int(&self) -> u32 {
    (u32::from(self.group) << 16) | u32::from(self.element)
  }

  /// Formats a data element tag as `"$GROUP$ELEMENT"`, e.g.`"0008002D"`.
  ///
  pub fn to_hex_string(&self) -> String {
    core::str::from_utf8(&self.to_hex_digits())
      .unwrap()
      .to_string()
  }

  /// Returns the eight hexadecimal digits for this data element tag's group
  /// and element values.
  ///
  pub fn to_hex_digits(&self) -> [u8; 8] {
    static HEX_DIGITS: &[u8; 16] = b"0123456789ABCDEF";

    [
      HEX_DIGITS[usize::from(self.group >> 12)],
      HEX_DIGITS[usize::from(self.group >> 8) & 0xF],
      HEX_DIGITS[usize::from(self.group >> 4) & 0xF],
      HEX_DIGITS[usize::from(self.group) & 0xF],
      HEX_DIGITS[usize::from(self.element >> 12)],
      HEX_DIGITS[usize::from(self.element >> 8) & 0xF],
      HEX_DIGITS[usize::from(self.element >> 4) & 0xF],
      HEX_DIGITS[usize::from(self.element) & 0xF],
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

  #[cfg(not(feature = "std"))]
  use alloc::format;

  #[test]
  fn debug_format_test() {
    assert_eq!(
      format!("{:?}", DataElementTag::new(0x7FE0, 0x0010)),
      "DataElementTag { group: 0x7FE0, element: 0x0010 }"
    );
  }

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
