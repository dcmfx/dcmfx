/// Describes a Value Length as stored in DICOM P10, which is either a defined
/// length containing a `u32` value, or an undefined length that is encoded as
/// `0xFFFFFFFF` in P10 data.
///
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueLength {
  Defined { length: u32 },
  Undefined,
}

impl ValueLength {
  /// Constructs a new value length from the given `u32` value. `0xFFFFFFFF` is
  /// an undefined length, all other values are a defined length.
  ///
  pub fn new(length: u32) -> Self {
    match length {
      0xFFFFFFFF => Self::Undefined,
      _ => Self::Defined { length },
    }
  }

  /// Convert a value length to a `u32`. An undefined length is `0xFFFFFFFF`
  /// and all defined lengths are just the contained length value.
  ///
  pub fn to_u32(self) -> u32 {
    match self {
      Self::Defined { length } => length,
      Self::Undefined => 0xFFFFFFFF,
    }
  }

  pub const ZERO: ValueLength = ValueLength::Defined { length: 0 };
}

impl core::fmt::Display for ValueLength {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      Self::Defined { length } => write!(f, "{} bytes", length),
      Self::Undefined => write!(f, "UNDEFINED"),
    }
  }
}
