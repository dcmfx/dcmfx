//// Work with the DICOM `PersonName` value representation.

import dcmfx_core/data_error.{type DataError}
import dcmfx_core/internal/bit_array_utils
import dcmfx_core/internal/utils
import gleam/bit_array
import gleam/bool
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string

/// The components of a single person name.
///
pub type PersonNameComponents {
  PersonNameComponents(
    last_name: String,
    first_name: String,
    middle_name: String,
    prefix: String,
    suffix: String,
  )
}

/// A structured person name that can be converted to/from a `PersonName` value.
/// Person name values have three variants: alphabetic, ideographic, and
/// phonetic. All variants are optional, however it is common for only the
/// alphabetic variant to be used.
///
pub type StructuredPersonName {
  StructuredPersonName(
    alphabetic: Option(PersonNameComponents),
    ideographic: Option(PersonNameComponents),
    phonetic: Option(PersonNameComponents),
  )
}

/// Converts a `PersonName` value to a list of structured person names.
///
pub fn from_bytes(
  bytes: BitArray,
) -> Result(List(StructuredPersonName), DataError) {
  let person_name_string =
    bytes
    |> bit_array.to_string
    |> result.map(utils.trim_right_whitespace)
    |> result.replace_error(data_error.new_value_invalid(
      "PersonName is invalid UTF-8",
    ))
  use person_name_string <- result.try(person_name_string)

  person_name_string
  |> string.split("\\")
  |> list.map(parse_person_name_string)
  |> result.all
}

/// Parses a `PersonName` value by splitting it on the '=' character to find the
/// list of component groups, then splitting each component group on
/// the '^' character to find the individual components of each name variant.
///
fn parse_person_name_string(
  person_name_string: String,
) -> Result(StructuredPersonName, DataError) {
  let component_groups = string.split(person_name_string, "=")

  let component_group_count = list.length(component_groups)

  let is_valid = component_group_count <= 3
  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid(
      "PersonName has too many component groups: "
      <> int.to_string(component_group_count),
    )),
  )

  let person_names =
    component_groups
    |> list.map(parse_person_name_component_group)
    |> result.all
  use person_names <- result.try(person_names)

  let #(alphabetic, ideographic, phonetic) = case person_names {
    [alphabetic] -> #(alphabetic, None, None)
    [alphabetic, ideographic] -> #(alphabetic, ideographic, None)
    [alphabetic, ideographic, phonetic] -> #(alphabetic, ideographic, phonetic)
    _ -> #(None, None, None)
  }

  Ok(StructuredPersonName(alphabetic, ideographic, phonetic))
}

fn parse_person_name_component_group(
  component_group: String,
) -> Result(Option(PersonNameComponents), DataError) {
  let components =
    string.split(component_group, "^")
    |> list.map(string.trim_right)

  let component_count = list.length(components)
  let is_valid = component_count > 0 && component_count <= 5

  use <- bool.guard(
    !is_valid,
    Error(data_error.new_value_invalid(
      "PersonName has too many components: " <> int.to_string(component_count),
    )),
  )

  // If all components of the name are empty then don't return anything
  let has_content = list.any(components, fn(component) { component != "" })
  use <- bool.guard(!has_content, Ok(None))

  // Append empty values to reach a list length of 5
  let components = list.append(components, list.repeat("", 5 - component_count))
  let assert [last_name, first_name, middle_name, prefix, suffix] = components

  Ok(
    Some(PersonNameComponents(
      last_name,
      first_name,
      middle_name,
      prefix,
      suffix,
    )),
  )
}

/// Converts a list of structured person names to a `PersonName` value.
///
pub fn to_bytes(
  value: List(StructuredPersonName),
) -> Result(BitArray, DataError) {
  let names =
    value
    |> list.map(fn(value) {
      [value.alphabetic, value.ideographic, value.phonetic]
      |> list.map(fn(component_group) {
        case component_group {
          Some(n) -> components_to_string(n)
          None -> Ok("")
        }
      })
      |> result.all
      |> result.map(string.join(_, "="))
      |> result.map(utils.trim_right(_, "="))
    })
    |> result.all

  use names <- result.map(names)

  names
  |> string.join("\\")
  |> bit_array.from_string
  |> bit_array_utils.pad_to_even_length(0x20)
}

fn components_to_string(
  components: PersonNameComponents,
) -> Result(String, DataError) {
  [
    components.last_name,
    components.first_name,
    components.middle_name,
    components.prefix,
    components.suffix,
  ]
  |> list.map(string.trim)
  |> list.map(fn(n) {
    // Check the maximum number of characters isn't exceeded
    let is_too_long = string.length(n) > 64
    use <- bool.guard(
      is_too_long,
      Error(data_error.new_value_invalid("PersonName component is too long")),
    )

    // Check there are no disallowed characters used
    let has_disallowed_characters =
      string.contains(n, "^")
      || string.contains(n, "=")
      || string.contains(n, "\\")

    use <- bool.guard(
      has_disallowed_characters,
      Error(data_error.new_value_invalid(
        "PersonName component has disallowed characters",
      )),
    )

    Ok(n)
  })
  |> result.all
  |> result.map(string.join(_, "^"))
  |> result.map(utils.trim_right(_, "^"))
}
