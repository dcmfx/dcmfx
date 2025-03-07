//! Work with the DICOM `AgeString` value representation.

#[cfg(not(feature = "std"))]
use alloc::{format, string::ToString, vec::Vec};

use regex::Regex;

use crate::DataError;

/// The time units that can be specified by a structured age.
///
#[derive(Clone, Debug, PartialEq)]
pub enum AgeUnit {
  Days,
  Weeks,
  Months,
  Years,
}

/// A structured age that can be converted to/from an `AgeString` value.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredAge {
  pub number: u16,
  pub unit: AgeUnit,
}

impl core::fmt::Display for StructuredAge {
  /// Formats a structured age as a human-readable string.
  ///
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    let unit = match self.unit {
      AgeUnit::Days => "day",
      AgeUnit::Weeks => "week",
      AgeUnit::Months => "month",
      AgeUnit::Years => "year",
    };

    let plural = if self.number == 1 { "" } else { "s" };

    write!(f, "{} {}{}", self.number, unit, plural)
  }
}

const PARSE_AGE_STRING_REGEX: &str = "^(\\d\\d\\d)([DWMY])$";

impl StructuredAge {
  /// Converts an `AgeString` value into a structured age.
  ///
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, DataError> {
    let age_string = core::str::from_utf8(bytes).map_err(|_| {
      DataError::new_value_invalid("AgeString is invalid UTF-8".to_string())
    })?;

    let age_string = age_string.trim_matches('\0').trim();

    match Regex::new(PARSE_AGE_STRING_REGEX)
      .unwrap()
      .captures(age_string)
    {
      Some(caps) => {
        let number = caps.get(1).unwrap().as_str().parse::<u16>().unwrap();
        let unit = caps.get(2).unwrap().as_str();

        let unit = match unit {
          "D" => AgeUnit::Days,
          "W" => AgeUnit::Weeks,
          "M" => AgeUnit::Months,
          _ => AgeUnit::Years,
        };

        Ok(Self { number, unit })
      }

      _ => Err(DataError::new_value_invalid(format!(
        "AgeString is invalid: '{}'",
        age_string
      ))),
    }
  }

  /// Converts a structured age into an `AgeString` value.
  ///
  pub fn to_bytes(&self) -> Result<Vec<u8>, DataError> {
    if self.number > 999 {
      return Err(DataError::new_value_invalid(format!(
        "AgeString value {} is outside the valid range of 0-999",
        self.number
      )));
    }

    let unit = match self.unit {
      AgeUnit::Days => "D",
      AgeUnit::Weeks => "W",
      AgeUnit::Months => "M",
      AgeUnit::Years => "Y",
    };

    Ok(format!("{:03}{}", self.number, unit).into_bytes())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      StructuredAge {
        number: 20,
        unit: AgeUnit::Days
      }
      .to_string(),
      "20 days"
    );

    assert_eq!(
      StructuredAge {
        number: 3,
        unit: AgeUnit::Weeks
      }
      .to_string(),
      "3 weeks"
    );

    assert_eq!(
      StructuredAge {
        number: 13,
        unit: AgeUnit::Months
      }
      .to_string(),
      "13 months"
    );

    assert_eq!(
      StructuredAge {
        number: 1,
        unit: AgeUnit::Years
      }
      .to_string(),
      "1 year"
    );
  }

  #[test]
  fn from_bytes_test() {
    assert_eq!(
      StructuredAge::from_bytes(b"101D"),
      Ok(StructuredAge {
        number: 101,
        unit: AgeUnit::Days
      })
    );

    assert_eq!(
      StructuredAge::from_bytes(b"070W"),
      Ok(StructuredAge {
        number: 70,
        unit: AgeUnit::Weeks
      })
    );

    assert_eq!(
      StructuredAge::from_bytes(b"009M"),
      Ok(StructuredAge {
        number: 9,
        unit: AgeUnit::Months
      })
    );

    assert_eq!(
      StructuredAge::from_bytes(b"101Y"),
      Ok(StructuredAge {
        number: 101,
        unit: AgeUnit::Years
      })
    );

    assert_eq!(
      StructuredAge::from_bytes(&[]),
      Err(DataError::new_value_invalid(
        "AgeString is invalid: ''".to_string()
      ))
    );

    assert_eq!(
      StructuredAge::from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "AgeString is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      StructuredAge::from_bytes(b"3 days"),
      Err(DataError::new_value_invalid(
        "AgeString is invalid: '3 days'".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(
      StructuredAge {
        number: 101,
        unit: AgeUnit::Days
      }
      .to_bytes(),
      Ok(b"101D".to_vec())
    );

    assert_eq!(
      StructuredAge {
        number: 70,
        unit: AgeUnit::Weeks
      }
      .to_bytes(),
      Ok(b"070W".to_vec())
    );

    assert_eq!(
      StructuredAge {
        number: 9,
        unit: AgeUnit::Months
      }
      .to_bytes(),
      Ok(b"009M".to_vec())
    );

    assert_eq!(
      StructuredAge {
        number: 101,
        unit: AgeUnit::Years
      }
      .to_bytes(),
      Ok(b"101Y".to_vec())
    );

    assert_eq!(
      StructuredAge {
        number: 1000,
        unit: AgeUnit::Years
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "AgeString value 1000 is outside the valid range of 0-999".to_string()
      )),
    );
  }
}
