//! Work with the DICOM `Date` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use regex::Regex;

use crate::DataError;

/// A structured date that can be converted to/from a `Date` value.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredDate {
  pub year: u16,
  pub month: u8,
  pub day: u8,
}

const PARSE_DATE_REGEX: &str = "^(\\d{4})(\\d\\d)(\\d\\d)$";

impl StructuredDate {
  /// Converts a `Date` value into a structured date.
  ///
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, DataError> {
    let date_string = core::str::from_utf8(bytes).map_err(|_| {
      DataError::new_value_invalid("Date is invalid UTF-8".to_string())
    })?;

    let date_string = date_string.trim_matches('\0').trim();

    match Regex::new(PARSE_DATE_REGEX).unwrap().captures(date_string) {
      Some(caps) => {
        let year = caps.get(1).unwrap().as_str().parse::<u16>().unwrap();
        let month = caps.get(2).unwrap().as_str().parse::<u8>().unwrap();
        let day = caps.get(3).unwrap().as_str().parse::<u8>().unwrap();

        Ok(Self { year, month, day })
      }

      _ => Err(DataError::new_value_invalid(format!(
        "Date is invalid: '{}'",
        date_string
      ))),
    }
  }

  /// Converts a structured date to a `Date` value.
  ///
  pub fn to_bytes(&self) -> Result<Vec<u8>, DataError> {
    Ok(
      Self::components_to_string(self.year, Some(self.month), Some(self.day))?
        .into_bytes(),
    )
  }

  /// Builds the content of a `Date` data element value where both the month and
  /// day are optional. The month value is required if there is a day specified.
  ///
  pub fn components_to_string(
    year: u16,
    month: Option<u8>,
    day: Option<u8>,
  ) -> Result<String, DataError> {
    let has_day_without_month = day.is_some() && month.is_none();
    if has_day_without_month {
      return Err(DataError::new_value_invalid(
        "Date's month must be present when there is a day value".to_string(),
      ));
    }

    // Validate and format the year value
    if year > 9999 {
      return Err(DataError::new_value_invalid(format!(
        "Date's year is invalid: {}",
        year
      )));
    }
    let year = format!("{:04}", year);

    // Validate and format the month value if present
    let month = match month {
      Some(month) => {
        if !(1..=12).contains(&month) {
          return Err(DataError::new_value_invalid(format!(
            "Date's month is invalid: {}",
            month
          )));
        }

        format!("{:02}", month)
      }

      None => "".to_string(),
    };

    // Validate and format the day value if present
    let day = match day {
      Some(day) => {
        if !(1..=31).contains(&day) {
          return Err(DataError::new_value_invalid(format!(
            "Date's day is invalid: {}",
            day
          )));
        }

        format!("{:02}", day)
      }

      None => "".to_string(),
    };

    Ok(format!("{}{}{}", year, month, day))
  }

  /// Formats a structured date as an ISO 8601 date.
  ///
  pub fn to_iso8601(&self) -> String {
    format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      StructuredDate {
        year: 2024,
        month: 7,
        day: 2
      }
      .to_iso8601(),
      "2024-07-02"
    );
  }

  #[test]
  fn from_bytes_test() {
    assert_eq!(
      StructuredDate::from_bytes(b"20000102"),
      Ok(StructuredDate {
        year: 2000,
        month: 1,
        day: 2
      })
    );

    assert_eq!(
      StructuredDate::from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "Date is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      StructuredDate::from_bytes(&[]),
      Err(DataError::new_value_invalid(
        "Date is invalid: ''".to_string()
      ))
    );

    assert_eq!(
      StructuredDate::from_bytes(b"2024"),
      Err(DataError::new_value_invalid(
        "Date is invalid: '2024'".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(
      StructuredDate {
        year: 2000,
        month: 1,
        day: 2
      }
      .to_bytes(),
      Ok(b"20000102".to_vec())
    );

    assert_eq!(
      StructuredDate {
        year: 10000,
        month: 1,
        day: 2
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Date's year is invalid: 10000".to_string()
      ))
    );

    assert_eq!(
      StructuredDate {
        year: 0,
        month: 13,
        day: 2
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Date's month is invalid: 13".to_string()
      ))
    );

    assert_eq!(
      StructuredDate {
        year: 100,
        month: 1,
        day: 32
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Date's day is invalid: 32".to_string()
      ))
    );
  }
}
