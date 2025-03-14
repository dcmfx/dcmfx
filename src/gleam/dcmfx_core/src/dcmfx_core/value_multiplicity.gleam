//// DICOM value multiplicity (VM).

import gleam/int
import gleam/option.{type Option, None, Some}

/// Describes DICOM value multiplicity, i.e. the number of values that are
/// allowed to be present in a data element. The `min` value is always at least
/// 1, and the maximum (if applicable) will always be greater than or equal to
/// `min`.
///
pub type ValueMultiplicity {
  ValueMultiplicity(min: Int, max: Option(Int))
}

/// Returns whether the given value lies in the range specified by this value
/// multiplicity.
///
pub fn contains(multiplicity: ValueMultiplicity, n: Int) -> Bool {
  n >= multiplicity.min && n <= option.unwrap(multiplicity.max, 0xFFFFFFFF)
}

/// Returns a value multiplicity as a human-readable string, e.g. "1-3", or
/// "2-n".
///
pub fn to_string(multiplicity: ValueMultiplicity) -> String {
  case multiplicity.min == 1 && multiplicity.max == Some(1) {
    True -> "1"
    False ->
      int.to_string(multiplicity.min)
      <> "-"
      <> case multiplicity.max {
        Some(max) -> int.to_string(max)
        None -> "n"
      }
  }
}
