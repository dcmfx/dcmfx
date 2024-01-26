//! DICOM value multiplicity.

/// Describes DICOM value multiplicity, where the multiplicity is the number of
/// values that are allowed to be present in a data element. The `min` value is
/// always at least 1, and the maximum (if applicable) will always be greater
/// than or equal to `min`.
///
#[derive(Clone, Debug, PartialEq)]
pub struct ValueMultiplicity {
  pub min: u32,
  pub max: Option<u32>,
}

impl std::fmt::Display for ValueMultiplicity {
  /// Returns a value multiplicity as a human-readable string, e.g. "1-3", or
  /// "2-n".
  ///
  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
    if self.min == 1 && self.max == Some(1) {
      return write!(f, "1");
    }

    let max = match self.max {
      Some(max) => max.to_string(),
      None => "n".to_string(),
    };

    write!(f, "{}-{}", self.min, max)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      ValueMultiplicity {
        min: 1,
        max: Some(1)
      }
      .to_string(),
      "1"
    );

    assert_eq!(
      ValueMultiplicity {
        min: 1,
        max: Some(3)
      }
      .to_string(),
      "1-3"
    );

    assert_eq!(ValueMultiplicity { min: 1, max: None }.to_string(), "1-n");
  }
}
