import dcmfx_json
import dcmfx_json/json_error
import dcmfx_p10
import dcmfx_p10/p10_error
import gleam/option.{None}
import glint
import simplifile

fn command_help() {
  "Converts a DICOM JSON file to a DICOM P10 file"
}

pub fn run() {
  use <- glint.command_help(command_help())
  use input_filename <- glint.named_arg("input-filename")
  use output_filename <- glint.named_arg("output-filename")
  use named_args, _, _ <- glint.command()

  let input_filename = input_filename(named_args)
  let output_filename = output_filename(named_args)

  case simplifile.read(input_filename) {
    Ok(json_data) ->
      case dcmfx_json.json_to_data_set(json_data) {
        Ok(data_set) ->
          case dcmfx_p10.write_file(output_filename, data_set, None) {
            Ok(_) -> Ok(Nil)

            Error(e) -> {
              p10_error.print(e, "writing file \"" <> output_filename <> "\"")
              Error(Nil)
            }
          }

        Error(e) -> {
          json_error.print_deserialize_error(
            e,
            "parsing file \"" <> input_filename <> "\"",
          )
          Error(Nil)
        }
      }

    Error(e) -> {
      p10_error.print(
        p10_error.FileError("Reading file", e),
        "reading file \"" <> input_filename <> "\"",
      )
      Error(Nil)
    }
  }
}
