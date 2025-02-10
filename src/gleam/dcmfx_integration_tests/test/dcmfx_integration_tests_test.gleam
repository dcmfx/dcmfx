import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_error
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import dcmfx_p10
import dcmfx_p10/data_set_builder.{type DataSetBuilder}
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_pixel_data
import dcmfx_pixel_data/pixel_data_filter.{type PixelDataFilterError}
import file_streams/file_stream.{type FileStream}
import file_streams/file_stream_error
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/json
import gleam/list
import gleam/option.{None}
import gleam/result
import gleam/string
import simplifile

pub fn main() {
  // List all files in the /test/assets/ directory
  let assert Ok(data_files) = simplifile.get_files("../../../test/assets")

  // Narrow down to just the DICOM files
  let dicoms =
    list.filter(data_files, string.ends_with(_, ".dcm"))
    |> list.sort(string.compare)

  // Validate each file
  io.print("Validating " <> int.to_string(list.length(dicoms)) <> " files … ")
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
            io.println("Error: No JSON file for \"" <> dicom <> "\"")

          Error(#(dicom, JsonOutputMismatch)) ->
            io.println(
              "Error: JSON mismatch on file \""
              <> dicom
              <> "\", compare the two files",
            )

          Error(#(dicom, RewriteMismatch)) ->
            io.println("Error: Rewrite of \"" <> dicom <> "\" was different")

          Error(#(dicom, JitteredReadError(error:))) ->
            p10_error.print(error, "reading " <> dicom <> " (jittered)")

          Error(#(dicom, JitteredReadMismatch)) ->
            io.println("Error: Jittered read of " <> dicom <> " was different")

          Error(#(dicom, PixelDataReadError(pixel_data_filter.DataError(error)))) ->
            data_error.print(error, "reading pixel data from " <> dicom)

          Error(#(dicom, PixelDataReadError(pixel_data_filter.P10Error(error)))) ->
            p10_error.print(error, "reading pixel data from " <> dicom)
        }
      })

      exit_with_status(1)
    }
  }
}

@external(erlang, "erlang", "halt")
@external(javascript, "node:process", "exit")
fn exit_with_status(status: Int) -> Nil

type DicomValidationError {
  LoadError(error: P10Error)
  JsonOutputMissing
  JsonOutputMismatch
  RewriteMismatch
  JitteredReadError(error: P10Error)
  JitteredReadMismatch
  PixelDataReadError(error: PixelDataFilterError)
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
    json.parse(expected_json_string, decode.dynamic)

  // Clean up DICOMs that have string values that consist only of spaces as
  // such values aren't preserved when going through a DICOM JSON rewrite
  // cycle
  let json_safe_data_set =
    data_set
    |> data_set.fold(data_set, fn(ds, tag, value) {
      case data_element_value.get_string(value) {
        Ok(s) ->
          case string.trim(s) {
            "" -> {
              let assert Ok(ds) =
                ds
                |> data_set.insert_binary_value(
                  tag,
                  data_element_value.value_representation(value),
                  <<>>,
                )
              ds
            }
            _ -> ds
          }
        Error(_) -> ds
      }
    })

  [
    test_data_set_matches_expected_json_output(
      dicom,
      json_safe_data_set,
      expected_json,
      False,
    ),
    test_data_set_matches_expected_json_output(
      dicom,
      json_safe_data_set,
      expected_json,
      True,
    ),
    test_dicom_json_rewrite_cycle(dicom, expected_json_string),
    test_dcmfx_p10_rewrite_cycle(dicom, data_set),
    test_jittered_read(dicom, data_set, fn() { 15 }),
    test_jittered_read(dicom, data_set, fn() { 1 + int.random(255) }),
    test_read_pixel_data(data_set),
  ]
  |> result.all
  |> result.replace(Nil)
}

/// Tests that the JSON conversion of the data set matches the expected JSON
/// content for the DICOM.
///
fn test_data_set_matches_expected_json_output(
  dicom: String,
  data_set: DataSet,
  expected_json: Dynamic,
  pretty_print: Bool,
) -> Result(Nil, DicomValidationError) {
  // Convert the data set to JSON
  let config =
    DicomJsonConfig(store_encapsulated_pixel_data: True, pretty_print:)
  let assert Ok(data_set_json) = dcmfx_json.data_set_to_json(data_set, config)
  let assert Ok(data_set_json) = json.parse(data_set_json, decode.dynamic)

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
    json.parse(expected_json_string, decode.dynamic)

  // Check the reverse by converting the expected JSON to a data set then back
  // to JSON and checking it matches the original. This tests the reading of
  // DICOM JSON data into a data set.
  let config =
    DicomJsonConfig(store_encapsulated_pixel_data: True, pretty_print: False)
  let assert Ok(data_set) = dcmfx_json.json_to_data_set(expected_json_string)
  let assert Ok(data_set_json_string) =
    dcmfx_json.data_set_to_json(data_set, config)
  let assert Ok(data_set_json) =
    json.parse(data_set_json_string, decode.dynamic)

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
    tag.group != 0x0002 && tag != dictionary.specific_character_set.tag
  }

  let data_set = data_set.filter(data_set, data_set_filter)
  let rewritten_data_set = data_set.filter(rewritten_data_set, data_set_filter)

  case data_set == rewritten_data_set {
    True -> Ok(Nil)
    False -> Error(RewriteMismatch)
  }
}

/// Reads a DICOM in streaming fashion with each chunk of incoming P10 data
/// being of a random size. This tests that DICOM reading is unaffected by
/// different input chunk sizes and where the boundaries between chunks fall.
///
fn test_jittered_read(
  dicom: String,
  data_set: DataSet,
  next_chunk_size: fn() -> Int,
) -> Result(Nil, DicomValidationError) {
  let assert Ok(stream) = file_stream.open_read(dicom)

  use builder <- result.try(test_jittered_read_loop(
    stream,
    p10_read.new_read_context(),
    data_set_builder.new(),
    next_chunk_size,
  ))

  case data_set_builder.final_data_set(builder) == Ok(data_set) {
    True -> Ok(Nil)
    False -> Error(JitteredReadMismatch)
  }
}

fn test_jittered_read_loop(
  file: FileStream,
  context: P10ReadContext,
  builder: DataSetBuilder,
  next_chunk_size: fn() -> Int,
) -> Result(DataSetBuilder, DicomValidationError) {
  case data_set_builder.is_complete(builder) {
    True -> Ok(builder)

    False -> {
      case p10_read.read_tokens(context) {
        Ok(#(tokens, context)) -> {
          let assert Ok(builder) =
            tokens
            |> list.try_fold(builder, fn(builder, token) {
              data_set_builder.add_token(builder, token)
            })

          test_jittered_read_loop(file, context, builder, next_chunk_size)
        }

        Error(p10_error.DataRequired(..)) -> {
          let assert Ok(context) = case
            file_stream.read_bytes(file, next_chunk_size())
          {
            Ok(bytes) -> p10_read.write_bytes(context, bytes, False)
            Error(file_stream_error.Eof) ->
              p10_read.write_bytes(context, <<>>, True)
            Error(e) -> panic as string.inspect(e)
          }

          test_jittered_read_loop(file, context, builder, next_chunk_size)
        }

        Error(error) -> Error(JitteredReadError(error:))
      }
    }
  }
}

/// Tests reading the frames of pixel data from a data set.
///
fn test_read_pixel_data(data_set: DataSet) -> Result(Nil, DicomValidationError) {
  case dcmfx_pixel_data.get_pixel_data_frames(data_set) {
    Ok(_) -> Ok(Nil)
    Error(e) -> Error(PixelDataReadError(e))
  }
}
