//! Work with the DICOM `Time` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use regex::Regex;

use crate::DataError;

/// A structured time that can be converted from/to a `Time` data element value.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredTime {
  pub hour: u8,
  pub minute: Option<u8>,
  pub second: Option<f64>,
}

const PARSE_TIME_REGEX: &str = "^(\\d\\d)((\\d\\d)((\\d\\d)(\\.\\d{1,6})?)?)?$";

impl StructuredTime {
  /// Converts a `Time` value into a structured time.
  ///
  pub fn from_bytes(bytes: &[u8]) -> Result<Self, DataError> {
    let time_string = core::str::from_utf8(bytes).map_err(|_| {
      DataError::new_value_invalid("Time is invalid UTF-8".to_string())
    })?;

    let time_string = time_string.trim_matches('\0').trim();

    match Regex::new(PARSE_TIME_REGEX).unwrap().captures(time_string) {
      Some(caps) => {
        let hour = caps.get(1).unwrap().as_str().parse::<u8>().unwrap();
        let minute = caps.get(3).map(|m| m.as_str().parse::<u8>().unwrap());
        let second = caps.get(4).map(|m| m.as_str().parse::<f64>().unwrap());

        Ok(StructuredTime {
          hour,
          minute,
          second,
        })
      }

      _ => Err(DataError::new_value_invalid(format!(
        "Time is invalid: '{time_string}'"
      ))),
    }
  }

  /// Converts a structured time to a `Time` value.
  ///
  pub fn to_bytes(&self) -> Result<Vec<u8>, DataError> {
    Ok(self.to_string()?.into_bytes())
  }

  /// Returns the string value of a structured time.
  ///
  pub fn to_string(&self) -> Result<String, DataError> {
    let has_second_without_minute =
      self.second.is_some() && self.minute.is_none();
    if has_second_without_minute {
      return Err(DataError::new_value_invalid(
        "Time minute value must be present when there is a second value"
          .to_string(),
      ));
    }

    // Validate and format the hour value
    if self.hour > 23 {
      return Err(DataError::new_value_invalid(format!(
        "Time hour value is invalid: {}",
        self.hour,
      )));
    }

    let hour = format!("{:02}", self.hour);

    // Validate and format the minute value if present
    let minute = match self.minute {
      Some(minute) => {
        if minute > 59 {
          return Err(DataError::new_value_invalid(format!(
            "Time minute value is invalid: {minute}"
          )));
        }

        format!("{minute:02}")
      }

      None => "".to_string(),
    };

    // Validate and format the second value if present. A second value of
    // exactly 60 is permitted in order to accommodate leap seconds.
    let second = match self.second {
      Some(second) => {
        if !(0.0..=60.0).contains(&second) {
          return Err(DataError::new_value_invalid(format!(
            "Time second value is invalid: {second}"
          )));
        }

        Self::format_second(second)
      }

      None => "".to_string(),
    };

    // Concatenate all the pieces of the time together
    Ok(format!("{hour}{minute}{second}"))
  }

  /// Formats a structured time as an ISO 8601 time. Components that aren't
  /// specified are omitted.
  ///
  pub fn to_iso8601(&self) -> String {
    let mut s = format!("{:02}", self.hour);

    if let Some(minute) = self.minute {
      s.push_str(&format!(":{minute:02}"));

      if let Some(second) = self.second {
        s.push(':');
        s.push_str(&Self::format_second(second));
      }
    }

    s
  }

  /// Takes a number of seconds and formats it as `SS[.FFFFFF]` with two digits
  /// for the whole number of seconds, and up to 6 digits for the fractional
  /// seconds. The fractional seconds are only included if the number of seconds
  /// is not an exact whole number.
  ///
  fn format_second(seconds: f64) -> String {
    let whole_seconds = format!("{:02}", seconds.floor() as u8);

    let fractional_seconds = (seconds.fract() * 1_000_000.0).round() as u32;

    if fractional_seconds == 0 {
      whole_seconds
    } else {
      let fractional_seconds = fractional_seconds.to_string();
      format!(
        "{}.{}",
        whole_seconds,
        fractional_seconds.trim_end_matches(['0'])
      )
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: Some(2),
        second: Some(3.289)
      }
      .to_iso8601(),
      "01:02:03.289"
    );

    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: Some(2),
        second: Some(3.0)
      }
      .to_iso8601(),
      "01:02:03"
    );

    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: Some(2),
        second: None
      }
      .to_iso8601(),
      "01:02"
    );

    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: None,
        second: None
      }
      .to_iso8601(),
      "01"
    );
  }

  #[test]
  fn from_bytes_test() {
    assert_eq!(
      StructuredTime::from_bytes(b"010203.289"),
      Ok(StructuredTime {
        hour: 1,
        minute: Some(2),
        second: Some(3.289)
      })
    );

    assert_eq!(
      StructuredTime::from_bytes(b"1115"),
      Ok(StructuredTime {
        hour: 11,
        minute: Some(15),
        second: None
      })
    );

    assert_eq!(
      StructuredTime::from_bytes(b"14"),
      Ok(StructuredTime {
        hour: 14,
        minute: None,
        second: None
      })
    );

    assert_eq!(
      StructuredTime::from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "Time is invalid UTF-8".to_string()
      ))
    );

    assert_eq!(
      StructuredTime::from_bytes(b"10pm"),
      Err(DataError::new_value_invalid(
        "Time is invalid: '10pm'".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: Some(2),
        second: Some(3.289)
      }
      .to_bytes(),
      Ok(b"010203.289".to_vec())
    );

    assert_eq!(
      StructuredTime {
        hour: 1,
        minute: Some(2),
        second: Some(3.0)
      }
      .to_bytes(),
      Ok(b"010203".to_vec())
    );

    assert_eq!(
      StructuredTime {
        hour: 23,
        minute: None,
        second: None
      }
      .to_bytes(),
      Ok(b"23".to_vec())
    );

    assert_eq!(
      StructuredTime {
        hour: 23,
        minute: Some(14),
        second: None
      }
      .to_bytes(),
      Ok(b"2314".to_vec())
    );

    assert_eq!(
      StructuredTime {
        hour: 23,
        minute: None,
        second: Some(1.0)
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Time minute value must be present when there is a second value"
          .to_string()
      ))
    );

    assert_eq!(
      StructuredTime {
        hour: 24,
        minute: None,
        second: None
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Time hour value is invalid: 24".to_string()
      ))
    );

    assert_eq!(
      StructuredTime {
        hour: 0,
        minute: Some(60),
        second: None
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Time minute value is invalid: 60".to_string()
      ))
    );

    assert_eq!(
      StructuredTime {
        hour: 0,
        minute: Some(0),
        second: Some(-1.2)
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Time second value is invalid: -1.2".to_string()
      ))
    );
  }
}
