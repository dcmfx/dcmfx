//! Shifting of dates for the Retain Longitudinal Temporal Information With
//! Modified Dates Option.

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

use dcmfx_core::{
  DataElementValue, StructuredDate, StructuredDateTime, StructuredTime,
  ValueRepresentation,
};

use crate::utils::trim_trailing_padding;

/// Shifts the dates in the given `Date`, `DateTime`, or `Time` value bytes by
/// the given number of days and returns the new value. Times are validated but
/// not modified. Multi-valued values have each of their values shifted.
///
/// Returns `None` if the VR is not a date or time VR, or if any value is
/// invalid or can't be shifted.
///
pub(crate) fn shift_value(
  vr: ValueRepresentation,
  bytes: &[u8],
  days: i32,
) -> Option<DataElementValue> {
  let values: Option<Vec<String>> = bytes
    .split(|b| *b == b'\\')
    .map(|value| {
      let value = trim_trailing_padding(value);

      if value.is_empty() {
        return Some(String::new());
      }

      match vr {
        ValueRepresentation::Date => shift_date(value, days),
        ValueRepresentation::DateTime => shift_date_time(value, days),
        ValueRepresentation::Time => StructuredTime::from_bytes(value)
          .ok()
          .map(|_| String::from_utf8_lossy(value).into_owned()),
        _ => None,
      }
    })
    .collect();

  let mut bytes = values?.join("\\").into_bytes();
  vr.pad_bytes_to_even_length(&mut bytes);

  Some(DataElementValue::new_binary_unchecked(vr, bytes.into()))
}

fn shift_date(bytes: &[u8], days: i32) -> Option<String> {
  let date = StructuredDate::from_bytes(bytes).ok()?;

  let (year, month, day) =
    shift_year_month_day(date.year, date.month, date.day, days)?;

  StructuredDate { year, month, day }
    .to_bytes()
    .ok()
    .and_then(|bytes| String::from_utf8(bytes).ok())
}

fn shift_date_time(bytes: &[u8], days: i32) -> Option<String> {
  let date_time = StructuredDateTime::from_bytes(bytes).ok()?;

  // A date/time with no month or day is shifted as if it were the first day of
  // the month or year, and its original precision is retained
  let (year, month, day) = shift_year_month_day(
    date_time.year,
    date_time.month.unwrap_or(1),
    date_time.day.unwrap_or(1),
    days,
  )?;

  // Only the date is rewritten. The time, fractional seconds, and time zone
  // offset that follow it are kept as they are so that their precision, such
  // as trailing zeros in the fractional seconds, is unchanged. The padding
  // that makes an odd-length date/time even is added once the values are
  // joined, not inside the joined value.
  let original = core::str::from_utf8(bytes).ok()?.trim_matches('\0').trim();

  let date = if date_time.day.is_some() {
    format!("{year:04}{month:02}{day:02}")
  } else if date_time.month.is_some() {
    format!("{year:04}{month:02}")
  } else {
    format!("{year:04}")
  };

  Some(date.clone() + original.get(date.len()..)?)
}

/// Shifts the given PTP timestamp, as used by Frame Origin Timestamp
/// (0034,0007), by the given number of days and returns the new value. The
/// timestamp is the 80-bit form defined by SMPTE ST 2110-10: 48 bits of
/// seconds followed by 32 bits of nanoseconds, both big-endian.
///
/// Returns `None` if the value isn't ten bytes long or the shifted seconds
/// don't fit in 48 bits.
///
pub(crate) fn shift_ptp_timestamp(bytes: &[u8], days: i32) -> Option<Vec<u8>> {
  if bytes.len() != 10 {
    return None;
  }

  let mut seconds = [0u8; 8];
  seconds[2..].copy_from_slice(&bytes[..6]);
  let seconds =
    i64::from_be_bytes(seconds).checked_add(i64::from(days) * 86400)?;

  if !(0..1 << 48).contains(&seconds) {
    return None;
  }

  let mut shifted = seconds.to_be_bytes()[2..].to_vec();
  shifted.extend_from_slice(&bytes[6..]);

  Some(shifted)
}

/// Shifts a date by the given number of days. Returns `None` if the date is
/// invalid or the shifted date falls outside the years 0 to 9999.
///
fn shift_year_month_day(
  year: u16,
  month: u8,
  day: u8,
  days: i32,
) -> Option<(u16, u8, u8)> {
  if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month)
  {
    return None;
  }

  let (year, month, day) = date_from_days(
    days_from_date(i64::from(year), i64::from(month), i64::from(day))
      + i64::from(days),
  );

  if !(0..=9999).contains(&year) {
    return None;
  }

  Some((year as u16, month as u8, day as u8))
}

fn days_in_month(year: u16, month: u8) -> u8 {
  match month {
    1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
    4 | 6 | 9 | 11 => 30,
    2 => {
      if year.is_multiple_of(4)
        && (!year.is_multiple_of(100) || year.is_multiple_of(400))
      {
        29
      } else {
        28
      }
    }
    _ => 0,
  }
}

/// Returns the number of days since 1970-01-01 for a date in the proleptic
/// Gregorian calendar. This and [`date_from_days()`] use the algorithms
/// described by Howard Hinnant at
/// <https://howardhinnant.github.io/date_algorithms.html>.
///
fn days_from_date(year: i64, month: i64, day: i64) -> i64 {
  let year = if month <= 2 { year - 1 } else { year };
  let era = if year >= 0 { year } else { year - 399 } / 400;
  let year_of_era = year - era * 400;
  let month_from_march = (month + 9) % 12;
  let day_of_year = (153 * month_from_march + 2) / 5 + day - 1;
  let day_of_era =
    year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

  era * 146097 + day_of_era - 719468
}

/// Returns the date in the proleptic Gregorian calendar for a number of days
/// since 1970-01-01.
///
fn date_from_days(days: i64) -> (i64, i64, i64) {
  let days = days + 719468;
  let era = if days >= 0 { days } else { days - 146096 } / 146097;
  let day_of_era = days - era * 146097;
  let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524
    - day_of_era / 146096)
    / 365;
  let year = year_of_era + era * 400;
  let day_of_year =
    day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
  let month_from_march = (5 * day_of_year + 2) / 153;
  let day = day_of_year - (153 * month_from_march + 2) / 5 + 1;
  let month = if month_from_march < 10 {
    month_from_march + 3
  } else {
    month_from_march - 9
  };

  (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
  use super::*;

  fn shift(vr: ValueRepresentation, s: &str, days: i32) -> Option<String> {
    shift_value(vr, s.as_bytes(), days)
      .map(|value| value.get_strings().unwrap().join("\\"))
  }

  #[test]
  fn shift_date_test() {
    let vr = ValueRepresentation::Date;

    assert_eq!(shift(vr, "20240301", -1), Some("20240229".to_string()));
    assert_eq!(shift(vr, "20230301", -1), Some("20230228".to_string()));
    assert_eq!(shift(vr, "20240102", -365), Some("20230102".to_string()));
    assert_eq!(shift(vr, "20241231", 1), Some("20250101".to_string()));
    assert_eq!(shift(vr, "20240102", 0), Some("20240102".to_string()));
    assert_eq!(shift(vr, "20240102 ", -1), Some("20240101".to_string()));
    assert_eq!(
      shift(vr, "20240102\\20240105", -1),
      Some("20240101\\20240104".to_string())
    );
    assert_eq!(shift(vr, "", -1), Some("".to_string()));

    assert_eq!(shift(vr, "2024", -1), None);
    assert_eq!(shift(vr, "20240230", -1), None);
    assert_eq!(shift(vr, "00000101", -1), None);
    assert_eq!(shift(vr, "99991231", 1), None);
    assert_eq!(shift(vr, "20240102\\invalid", -1), None);
  }

  #[test]
  fn shift_date_time_test() {
    let vr = ValueRepresentation::DateTime;

    assert_eq!(
      shift(vr, "20240301123456.5+1200", -1),
      Some("20240229123456.5+1200".to_string())
    );
    assert_eq!(shift(vr, "20240301", -1), Some("20240229".to_string()));
    assert_eq!(shift(vr, "202403", -1), Some("202402".to_string()));
    assert_eq!(shift(vr, "2024", -1), Some("2023".to_string()));
    assert_eq!(shift(vr, "2024", 1), Some("2024".to_string()));

    assert_eq!(shift(vr, "2024-01", -1), None);

    // The precision of the time that follows the date is unchanged
    assert_eq!(
      shift(vr, "20240315102401.000000", -7),
      Some("20240308102401.000000".to_string())
    );
    assert_eq!(
      shift(vr, "20240315102401.100000+0100", -7),
      Some("20240308102401.100000+0100".to_string())
    );

    // An odd-length value is padded once the values are joined, not inside
    // the joined value
    assert_eq!(
      shift_value(vr, b"20240315123456.12\\20240316123456.12 ", -10)
        .unwrap()
        .bytes()
        .unwrap()
        .as_ref(),
      b"20240305123456.12\\20240306123456.12 "
    );
  }

  #[test]
  fn shift_time_test() {
    let vr = ValueRepresentation::Time;

    assert_eq!(shift(vr, "123456", -1), Some("123456".to_string()));
    assert_eq!(shift(vr, "1234", 100), Some("1234".to_string()));
    assert_eq!(shift(vr, "12:34", 100), None);
  }

  #[test]
  fn other_vrs_are_not_shifted() {
    assert_eq!(shift(ValueRepresentation::ShortString, "+1200", -1), None);
  }

  #[test]
  fn days_from_date_round_trip() {
    for days in [-1_000_000, -719468, -1, 0, 1, 19723, 2_932_896] {
      let (year, month, day) = date_from_days(days);
      assert_eq!(days_from_date(year, month, day), days);
    }

    assert_eq!(days_from_date(1970, 1, 1), 0);
    assert_eq!(date_from_days(19723), (2024, 1, 1));
  }
}
