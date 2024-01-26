import dcmfx_core/data_element_tag
import dcmfx_core/data_error
import dcmfx_core/data_set_path.{type DataSetPath}
import dcmfx_core/registry
import gleam/io
import gleam/list
import gleam/option.{None}

/// Occurs when an error is encountered converting to the DICOM JSON model.
///
pub type JsonSerializeError {
  /// The data to be serialized to the DICOM JSON model is invalid. Details of
  /// the issue are contained in the enclosed [`DataError`].
  DataError(data_error: data_error.DataError)
}

/// Occurs when an error is encountered converting from the DICOM JSON model.
///
pub type JsonDeserializeError {
  /// The DICOM JSON data to be deserialized is invalid.
  JsonInvalid(details: String, path: DataSetPath)
}

/// Returns lines of output that describe a DICOM JSON serialize error in a
/// human-readable format.
///
pub fn serialize_error_to_lines(
  error: JsonSerializeError,
  task_description: String,
) -> List(String) {
  case error {
    DataError(error) -> data_error.to_lines(error, task_description)
  }
}

/// Returns lines of output that describe a DICOM JSON deserialize error in a
/// human-readable format.
///
pub fn deserialize_error_to_lines(
  error: JsonDeserializeError,
  task_description: String,
) -> List(String) {
  case error {
    JsonInvalid(details:, path:) -> {
      list.concat([
        [
          "DICOM JSON deserialize error " <> task_description,
          "",
          "  Details: " <> details,
        ],
        case data_set_path.final_data_element(path) {
          Ok(tag) -> [
            "  Tag: " <> data_element_tag.to_string(tag),
            "  Name: " <> registry.tag_name(tag, None),
          ]

          _ -> []
        },
        case data_set_path.is_empty(path) {
          True -> []
          False -> ["  Path: " <> data_set_path.to_string(path)]
        },
      ])
    }
  }
}

/// Prints a DICOM JSON serialize error to stderr in a human-readable format.
///
pub fn print_serialize_error(
  error: JsonSerializeError,
  task_description: String,
) {
  serialize_error_to_lines(error, task_description)
  |> list.each(io.println)
}

/// Prints a DICOM JSON deserialize error to stderr in a human-readable format.
///
pub fn print_deserialize_error(
  error: JsonDeserializeError,
  task_description: String,
) {
  deserialize_error_to_lines(error, task_description)
  |> list.each(io.println)
}
