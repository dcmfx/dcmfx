//! Work with the DICOM `DateTime` value representation.

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
  vec::Vec,
};

use regex::Regex;

use crate::data_element_value::date::StructuredDate;
use crate::{DataError, StructuredTime};

/// A structured date/time that can be converted to/from a `DateTime` value.
///
#[derive(Clone, Debug, PartialEq)]
pub struct StructuredDateTime {
  pub year: u16,
  pub month: Option<u8>,
  pub day: Option<u8>,
  pub hour: Option<u8>,
  pub minute: Option<u8>,
  pub second: Option<f64>,
  pub time_zone_offset: Option<i16>,
}

const PARSE_DATE_TIME_REGEX: &str = "^(\\d{4})((\\d{2})((\\d{2})((\\d{2})((\\d{2})((\\d{2})(\\.\\d{1,6})?)?)?)?)?)?([\\+\\-]\\d{4})?$";

impl StructuredDateTime {
  /// Converts a `DateTime` value into a structured date/time.
  ///
  pub fn from_bytes(bytes: &[u8]) -> Result<StructuredDateTime, DataError> {
    let date_time_string = core::str::from_utf8(bytes).map_err(|_| {
      DataError::new_value_invalid("DateTime is invalid UTF-8".to_string())
    })?;

    let date_time_string = date_time_string.trim_matches('\0').trim();

    match Regex::new(PARSE_DATE_TIME_REGEX)
      .unwrap()
      .captures(date_time_string)
    {
      Some(caps) => {
        let year = caps.get(1).unwrap().as_str().parse::<u16>().unwrap();
        let month = caps.get(3).map(|m| m.as_str().parse::<u8>().unwrap());
        let day = caps.get(5).map(|d| d.as_str().parse::<u8>().unwrap());
        let hour = caps.get(7).map(|h| h.as_str().parse::<u8>().unwrap());
        let minute = caps.get(9).map(|m| m.as_str().parse::<u8>().unwrap());
        let second = caps.get(10).map(|s| s.as_str().parse::<f64>().unwrap());
        let time_zone_offset =
          caps.get(13).map(|o| o.as_str().parse::<i16>().unwrap());

        Ok(StructuredDateTime {
          year,
          month,
          day,
          hour,
          minute,
          second,
          time_zone_offset,
        })
      }

      _ => Err(DataError::new_value_invalid(format!(
        "DateTime is invalid: '{date_time_string}'"
      ))),
    }
  }

  /// Converts a structured date/time to a `DateTime` value.
  ///
  pub fn to_bytes(&self) -> Result<Vec<u8>, DataError> {
    let has_hour_without_day = self.hour.is_some() && self.day.is_none();
    if has_hour_without_day {
      return Err(DataError::new_value_invalid(
        "DateTime day value must be present when there is an hour value"
          .to_string(),
      ));
    }

    // Validate and format the date
    let date =
      StructuredDate::components_to_string(self.year, self.month, self.day)?;

    // Validate and format the time if present
    let time = match self.hour {
      Some(hour) => StructuredTime {
        hour,
        minute: self.minute,
        second: self.second,
      }
      .to_string(),
      _ => Ok("".to_string()),
    }?;

    // Validate and format the time zone offset if present
    let time_zone_offset = match self.time_zone_offset {
      Some(offset) => {
        let is_offset_valid =
          (-1200..=1400).contains(&offset) && (offset % 100 < 60);

        if !is_offset_valid {
          return Err(DataError::new_value_invalid(format!(
            "DateTime time zone offset is invalid: {offset}"
          )));
        }

        let sign = if offset < 0 { "-" } else { "+" };

        format!("{}{:04}", sign, offset.abs())
      }

      None => "".to_string(),
    };

    let mut bytes = format!("{date}{time}{time_zone_offset}").into_bytes();

    if bytes.len() % 2 == 1 {
      bytes.push(0x20);
    }

    Ok(bytes)
  }

  /// Formats a structured date/time as an ISO 8601 string. Components that
  /// aren't specified are omitted.
  ///
  pub fn to_iso8601(&self) -> String {
    let mut s = format!("{:04}", self.year);

    if let Some(month) = self.month {
      s.push_str(&format!("-{month:02}"));

      if let Some(day) = self.day {
        s.push_str(&format!("-{day:02}"));

        if let Some(hour) = self.hour {
          let time = StructuredTime {
            hour,
            minute: self.minute,
            second: self.second,
          };

          s.push('T');
          s.push_str(&time.to_iso8601());
        }
      }
    }

    if let Some(time_zone_offset) = self.time_zone_offset {
      s.push(if time_zone_offset < 0 { '-' } else { '+' });
      s.push_str(&format!("{:04}", time_zone_offset.abs()));
    }

    s
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn to_string_test() {
    assert_eq!(
      StructuredDateTime {
        year: 2024,
        month: Some(7),
        day: Some(2),
        hour: Some(9),
        minute: Some(40),
        second: Some(2.5),
        time_zone_offset: Some(-400)
      }
      .to_iso8601(),
      "2024-07-02T09:40:02.5-0400"
    );

    assert_eq!(
      StructuredDateTime {
        year: 2024,
        month: Some(7),
        day: Some(2),
        hour: Some(9),
        minute: None,
        second: None,
        time_zone_offset: Some(200)
      }
      .to_iso8601(),
      "2024-07-02T09+0200"
    );
  }

  #[test]
  fn from_bytes_test() {
    assert_eq!(
      StructuredDateTime::from_bytes(b"1997"),
      Ok(StructuredDateTime {
        year: 1997,
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        time_zone_offset: None
      })
    );

    assert_eq!(
      StructuredDateTime::from_bytes(b"1997070421-0500"),
      Ok(StructuredDateTime {
        year: 1997,
        month: Some(7),
        day: Some(4),
        hour: Some(21),
        minute: None,
        second: None,
        time_zone_offset: Some(-500)
      })
    );

    assert_eq!(
      StructuredDateTime::from_bytes(b"19970704213000-0500"),
      Ok(StructuredDateTime {
        year: 1997,
        month: Some(7),
        day: Some(4),
        hour: Some(21),
        minute: Some(30),
        second: Some(0.0),
        time_zone_offset: Some(-500)
      })
    );

    assert_eq!(
      StructuredDateTime::from_bytes(b"10pm"),
      Err(DataError::new_value_invalid(
        "DateTime is invalid: '10pm'".to_string()
      ))
    );

    assert_eq!(
      StructuredDateTime::from_bytes(&[0xD0]),
      Err(DataError::new_value_invalid(
        "DateTime is invalid UTF-8".to_string()
      ))
    );
  }

  #[test]
  fn to_bytes_test() {
    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: Some(7),
        day: Some(4),
        hour: Some(21),
        minute: Some(30),
        second: Some(0.0),
        time_zone_offset: Some(-500),
      }
      .to_bytes(),
      Ok(b"19970704213000-0500 ".to_vec())
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: Some(7),
        day: Some(4),
        hour: None,
        minute: None,
        second: None,
        time_zone_offset: None,
      }
      .to_bytes(),
      Ok(b"19970704".to_vec())
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        time_zone_offset: Some(100),
      }
      .to_bytes(),
      Ok(b"1997+0100 ".to_vec())
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: Some(1),
        day: None,
        hour: Some(1),
        minute: None,
        second: None,
        time_zone_offset: None,
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "DateTime day value must be present when there is an hour value"
          .to_string()
      ))
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: None,
        day: Some(1),
        hour: None,
        minute: None,
        second: None,
        time_zone_offset: None,
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Date's month must be present when there is a day value".to_string()
      ))
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: Some(1),
        day: Some(1),
        hour: Some(30),
        minute: None,
        second: None,
        time_zone_offset: None,
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "Time hour value is invalid: 30".to_string()
      ))
    );

    assert_eq!(
      StructuredDateTime {
        year: 1997,
        month: None,
        day: None,
        hour: None,
        minute: None,
        second: None,
        time_zone_offset: Some(2000),
      }
      .to_bytes(),
      Err(DataError::new_value_invalid(
        "DateTime time zone offset is invalid: 2000".to_string()
      ))
    );
  }
}
