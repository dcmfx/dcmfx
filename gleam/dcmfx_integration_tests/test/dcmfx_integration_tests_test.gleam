import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/registry
import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import gleam/dynamic.{type Dynamic}
import gleam/int
import gleam/io
import gleam/json
import gleam/list
import gleam/option.{None}
import gleam/result
import gleam/string
import simplifile

pub fn main() {
  // List all files in the test_assets/ directory
  let assert Ok(data_files) = simplifile.get_files("../../test_assets")

  // Narrow down to just the DICOM files
  let dicoms =
    list.filter(data_files, string.ends_with(_, ".dcm"))
    |> list.sort(string.compare)

  // Validate each file
  io.print("Validating " <> int.to_string(list.length(dicoms)) <> " files ... ")
  let validation_results =
    list.map(dicoms, fn(dicom) {
      dicom
      |> validate_dicom
      |> result.map_error(fn(e) { #(dicom, e) })
    })

  // Print results
  case result.all(validation_results) {
    Ok(_) -> io.println("✅\n")

    // Report details on failures
    Error(_) -> {
      io.println("❌\n")

      list.each(validation_results, fn(task_result) {
        case task_result {
          Ok(Nil) -> Nil

          Error(#(dicom, LoadError(e))) ->
            p10_error.print(e, "reading " <> dicom)

          Error(#(dicom, JsonOutputMissing)) ->
            io.println("Error: No JSON file for dicom \"" <> dicom <> "\"")

          Error(#(dicom, JsonOutputMismatch)) ->
            io.println(
              "Error: JSON mismatch on file \""
              <> dicom
              <> "\", compare the two files for details",
            )

          Error(#(dicom, RewriteMismatch)) ->
            io.println(
              "Error: Rewrite of file \"" <> dicom <> "\" was different",
            )
        }
      })

      exit(1)
    }
  }
}

@external(erlang, "erlang", "halt")
@external(javascript, "node:process", "exit")
fn exit(status: Int) -> Nil

type DicomValidationError {
  LoadError(error: P10Error)
  JsonOutputMissing
  JsonOutputMismatch
  RewriteMismatch
}

/// Loads a DICOM file and checks that its JSON serialization by this library
/// matches the expected JSON serialization stored alongside it on disk.
///
fn validate_dicom(dicom: String) -> Result(Nil, DicomValidationError) {
  // Load the DICOM
  let data_set =
    dcmfx_p10.read_file(dicom)
    |> result.map_error(LoadError)
  use data_set <- result.try(data_set)

  // Read the expected JSON output from the associated .json file
  let expected_json_string =
    simplifile.read(dicom <> ".json")
    |> result.replace_error(JsonOutputMissing)
  use expected_json_string <- result.try(expected_json_string)

  let assert Ok(expected_json) =
    json.decode(expected_json_string, dynamic.dynamic)

  // Run several tests on the data set
  [
    test_data_set_matches_expected_json(dicom, data_set, expected_json),
    test_dicom_json_rewrite_cycle(dicom, expected_json_string),
    test_dcmfx_p10_rewrite_cycle(dicom, data_set),
  ]
  |> result.all
  |> result.replace(Nil)
}

/// Tests that the JSON conversion of the data set matches the expected JSON
/// content for the DICOM.
///
fn test_data_set_matches_expected_json(
  dicom: String,
  data_set: DataSet,
  expected_json: Dynamic,
) -> Result(Nil, DicomValidationError) {
  // Convert the data set to JSON
  let config = DicomJsonConfig(store_encapsulated_pixel_data: True)
  let assert Ok(data_set_json) = dcmfx_json.data_set_to_json(data_set, config)
  let assert Ok(data_set_json) = json.decode(data_set_json, dynamic.dynamic)

  // Compare the actual JSON to the expected JSON
  case data_set_json == expected_json {
    True -> Ok(Nil)

    False -> {
      // The JSON didn't match so write what was generated to a separate JSON
      // file so it can be manually compared to find the discrepancy
      let assert Ok(data_set_json) =
        dcmfx_json.data_set_to_json(data_set, config)
      let assert Ok(Nil) =
        simplifile.write(dicom <> ".validation_failure.json", data_set_json)

      Error(JsonOutputMismatch)
    }
  }
}

/// Tests that the conversion of the given DICOM JSON content is unchanged when
/// converted to a data set and then converted back to DICOM JSON.
///
fn test_dicom_json_rewrite_cycle(
  dicom: String,
  expected_json_string: String,
) -> Result(Nil, DicomValidationError) {
  let assert Ok(original_json) =
    json.decode(expected_json_string, dynamic.dynamic)

  // Check the reverse by converting the expected JSON to a data set then back
  // to JSON and checking it matches the original. This tests the reading of
  // DICOM JSON data into a data set.
  let config = DicomJsonConfig(store_encapsulated_pixel_data: True)
  let assert Ok(data_set) = dcmfx_json.json_to_data_set(expected_json_string)
  let assert Ok(data_set_json_string) =
    dcmfx_json.data_set_to_json(data_set, config)
  let assert Ok(data_set_json) =
    json.decode(data_set_json_string, dynamic.dynamic)

  // Compare the actual JSON to the expected JSON
  case original_json == data_set_json {
    True -> Ok(Nil)

    False -> {
      // The JSON didn't match so write what was generated to a separate JSON
      // file so it can be manually compared to find the discrepancy
      let assert Ok(Nil) =
        simplifile.write(
          dicom <> ".validation_failure.json",
          data_set_json_string,
        )

      Error(JsonOutputMismatch)
    }
  }
}

/// Puts a data set through a full write and read cycle and checks that nothing
/// changes.
///
fn test_dcmfx_p10_rewrite_cycle(
  dicom: String,
  data_set: DataSet,
) -> Result(Nil, DicomValidationError) {
  let assert Ok(_) = dcmfx_p10.write_file(dicom <> ".tmp", data_set, None)
  let assert Ok(rewritten_data_set) = dcmfx_p10.read_file(dicom <> ".tmp")
  let assert Ok(_) = simplifile.delete(dicom <> ".tmp")

  // Filter that removes File Meta Information and specific character set data
  // elements which we don't want to be part of the rewrite comparison
  let data_set_filter = fn(tag: DataElementTag, _value) {
    tag.group != 0x0002 && tag != registry.specific_character_set.tag
  }

  let data_set = data_set.filter(data_set, data_set_filter)
  let rewritten_data_set = data_set.filter(rewritten_data_set, data_set_filter)

  case data_set == rewritten_data_set {
    True -> Ok(Nil)
    False -> Error(RewriteMismatch)
  }
}
