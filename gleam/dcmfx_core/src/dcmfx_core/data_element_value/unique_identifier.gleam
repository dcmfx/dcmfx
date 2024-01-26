//// Work with the DICOM `UniqueIdentifier` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/list
import gleam/regex
import gleam/result
import gleam/string

/// Converts a list of UIDs into a `UniqueIdentifier` value.
///
pub fn to_bytes(uids: List(String)) -> Result(BitArray, DataError) {
  uids
  |> list.map(fn(uid) {
    case is_valid(uid) {
      True -> Ok(uid)
      False ->
        Error(data_error.new_value_invalid("UniqueIdentifier is invalid"))
    }
  })
  |> result.all
  |> result.map(string.join(_, "\\"))
  |> result.map(bit_array.from_string)
  |> result.map(bit_array_utils.pad_to_even_length(_, 0x00))
}

/// Returns whether the given string is a valid `UniqueIdentifier`. Valid UIDs
/// are 1-64 characters in length, and are made up of sequences of digits
/// separated by the period character. Leading zeros are not permitted in a
/// digit sequence unless the zero is the only digit in the sequence.
///
pub fn is_valid(uid: String) -> Bool {
  let length = utils.string_fast_length(uid)

  // Check the length is valid
  use <- bool.guard(length == 0 || length > 64, False)

  let assert Ok(re) =
    regex.from_string("^(0|[1-9][0-9]*)(\\.(0|[1-9][0-9]*))*$")

  regex.check(re, uid)
}

/// Generates a new random UID with the given prefix. The new UID will have a
/// length of 64 characters. The maximum prefix length is 60, and if specified
/// it must end with a '.' character.
///
pub fn new(prefix: String) -> Result(String, Nil) {
  let prefix_length = utils.string_fast_length(prefix)

  // Check the prefix is valid
  use <- bool.guard(
    prefix_length > 60 || prefix_length > 0 && !is_valid(prefix),
    Error(Nil),
  )

  // Start with a non-zero character
  let uid =
    case prefix_length {
      0 -> ""
      _ -> prefix <> "."
    }
    <> random_character(49, 9)

  do_new(uid, 64 - utils.string_fast_length(uid)) |> Ok
}

fn do_new(uid: String, remaining_digits: Int) -> String {
  case remaining_digits {
    0 -> uid
    _ -> do_new(uid <> random_character(48, 10), remaining_digits - 1)
  }
}

fn random_character(offset: Int, range: Int) {
  let assert Ok(cp) = string.utf_codepoint(offset + int.random(range))

  string.from_utf_codepoints([cp])
}
