//// Provides the `DataError` type that describes the errors that can occur when
//// working with data sets and elements.

import dcmfx_core/data_element_tag
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/dictionary
import dcmfx_core/value_representation.{type ValueRepresentation}
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}

/// An error that occurred when retrieving or creating data elements.
///
pub opaque type DataError {
  /// When retrieving a value, the requested tag was not present in the data
  /// set.
  TagNotPresent(path: DataSetPath)

  /// When retrieving a value, the requested type is not present. E.g. tried to
  /// retrieve an integer value when the data element value contains a string.
  ///
  ValueNotPresent(path: Option(DataSetPath))

  /// When retrieving a value, it did not have the required multiplicity. E.g.
  /// tried to retrieve a single string value when the data element contained
  /// multiple string values.
  ///
  MultiplicityMismatch(path: Option(DataSetPath))

  /// When retrieving a value, there was an error decoding its bytes. E.g. a
  /// string value that had bytes that are not valid UTF-8, or a `PersonName`
  /// value that had an invalid structure.
  ///
  /// When creating a value, the supplied input was not valid for the type of
  /// data element being created.
  ValueInvalid(details: String, path: Option(DataSetPath))

  /// When creating a value, the supplied data did not meet a required length
  /// constraint, e.g. the minimum or maximum length for the value
  /// representation wasn't respected.
  ValueLengthInvalid(
    vr: ValueRepresentation,
    length: Int,
    details: String,
    path: Option(DataSetPath),
  )

  /// When creating, reading, or parsing a value, the value itself is valid but
  /// is not supported by this library.
  ValueUnsupported(details: String, path: Option(DataSetPath))
}

/// Converts a data error a human-readable string.
///
pub fn to_string(error: DataError) -> String {
  let optional_path_to_string = fn(path: Option(DataSetPath)) -> String {
    path
    |> option.map(data_set_path.to_detailed_string)
    |> option.unwrap("<unknown>")
  }

  "DICOM Data Error: "
  <> case error {
    TagNotPresent(path) ->
      "Tag not present at " <> data_set_path.to_detailed_string(path)
    ValueNotPresent(path) ->
      "Value not present at " <> optional_path_to_string(path)
    MultiplicityMismatch(path) ->
      "Multiplicity mismatch at " <> optional_path_to_string(path)
    ValueInvalid(details, path) ->
      "Invalid value at "
      <> optional_path_to_string(path)
      <> ", details: "
      <> details
    ValueLengthInvalid(details:, path:, ..) ->
      "Invalid value length at "
      <> optional_path_to_string(path)
      <> ", details: "
      <> details
    ValueUnsupported(details:, ..) -> "Value unsupported, details: " <> details
  }
}

/// Constructs a new 'Tag not present' data error.
///
pub fn new_tag_not_present() -> DataError {
  TagNotPresent(path: data_set_path.new())
}

/// Constructs a new 'Value not present' data error.
///
pub fn new_value_not_present() -> DataError {
  ValueNotPresent(path: None)
}

/// Constructs a new 'Multiplicity mismatch' data error.
///
pub fn new_multiplicity_mismatch() -> DataError {
  MultiplicityMismatch(path: None)
}

/// Constructs a new 'Value invalid' data error.
///
pub fn new_value_invalid(details: String) -> DataError {
  ValueInvalid(details, None)
}

/// Constructs a new 'Value length invalid' data error.
///
pub fn new_value_length_invalid(
  vr: ValueRepresentation,
  length: Int,
  details: String,
) -> DataError {
  ValueLengthInvalid(vr, length, details, None)
}

/// Constructs a new 'Value not supported' data error.
///
pub fn new_value_unsupported(details: String) -> DataError {
  ValueUnsupported(details, None)
}

/// Returns the data set path for a data error.
///
pub fn path(error: DataError) -> Option(DataSetPath) {
  case error {
    TagNotPresent(path:) -> Some(path)
    ValueNotPresent(path:)
    | MultiplicityMismatch(path:)
    | ValueInvalid(path:, ..)
    | ValueLengthInvalid(path:, ..)
    | ValueUnsupported(path:, ..) -> path
  }
}

/// Returns whether a data error is a 'Tag not present' error.
///
pub fn is_tag_not_present(error: DataError) -> Bool {
  case error {
    TagNotPresent(..) -> True
    _ -> False
  }
}

/// Adds a data set path to a data error. This indicates the exact location
/// that a data error occurred in a data set, and should be included wherever
/// possible to make troubleshooting easier.
///
pub fn with_path(error: DataError, path: DataSetPath) -> DataError {
  case error {
    TagNotPresent(..) -> TagNotPresent(path)
    ValueNotPresent(..) -> ValueNotPresent(Some(path))
    MultiplicityMismatch(..) -> ValueNotPresent(Some(path))
    ValueInvalid(details, ..) -> ValueInvalid(details, Some(path))
    ValueLengthInvalid(vr, length, details, ..) ->
      ValueLengthInvalid(vr, length, details, Some(path))
    ValueUnsupported(details:, ..) -> ValueUnsupported(details, Some(path))
  }
}

/// Returns the name of a data error as a human-readable string.
///
pub fn name(error: DataError) -> String {
  case error {
    TagNotPresent(..) -> "Tag not present"
    ValueNotPresent(..) -> "Value not present"
    MultiplicityMismatch(..) -> "Multiplicity mismatch"
    ValueInvalid(..) -> "Invalid value"
    ValueLengthInvalid(..) -> "Invalid value length"
    ValueUnsupported(..) -> "Unsupported value"
  }
}

/// Returns lines of output that describe a DICOM data error in a human-readable
/// format.
///
pub fn to_lines(error: DataError, task_description: String) -> List(String) {
  let initial_lines = [
    "DICOM data error " <> task_description,
    "",
    "  Error: " <> name(error),
  ]

  let path_lines = case error {
    TagNotPresent(path)
    | ValueNotPresent(Some(path))
    | MultiplicityMismatch(Some(path))
    | ValueInvalid(_, Some(path))
    | ValueLengthInvalid(_, _, _, Some(path)) -> {
      let path_line = "  Path: " <> data_set_path.to_string(path)

      case data_set_path.final_data_element(path) {
        Ok(tag) -> [
          "  Tag: " <> data_element_tag.to_string(tag),
          "  Name: " <> dictionary.tag_name(tag, None),
          path_line,
        ]
        _ -> [path_line]
      }
    }
    _ -> []
  }

  let details_lines = case error {
    ValueInvalid(details, _) -> ["  Details: " <> details]
    ValueLengthInvalid(vr, length, details, _) -> [
      "  VR: " <> value_representation.to_string(vr),
      "  Length: " <> int.to_string(length) <> " bytes",
      "  Details: " <> details,
    ]
    ValueUnsupported(details:, ..) -> ["  Details: " <> details]
    _ -> []
  }

  list.flatten([initial_lines, path_lines, details_lines])
}

/// Prints a DICOM data error to stderr in a human-readable format.
///
pub fn print(error: DataError, task_description: String) -> Nil {
  io.println_error("")
  io.println_error("-----")

  error
  |> to_lines(task_description)
  |> list.each(io.println_error)

  io.println_error("")
}
