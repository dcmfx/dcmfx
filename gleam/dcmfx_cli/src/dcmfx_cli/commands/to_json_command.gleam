import dcmfx_core/data_error
import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import dcmfx_json/json_error
import dcmfx_p10
import dcmfx_p10/p10_error
import glint
import simplifile

fn command_help() {
  "Converts a DICOM P10 file to a DICOM JSON file"
}

fn store_encapsulated_pixel_data_flag() {
  glint.bool_flag("store-encapsulated-pixel-data")
  |> glint.flag_default(False)
  |> glint.flag_help(
    "Whether to extend the DICOM JSON model to store encapsulated pixel data "
    <> "as inline binaries",
  )
}

pub fn run() {
  use <- glint.command_help(command_help())
  use input_filename <- glint.named_arg("input-filename")
  use output_filename <- glint.named_arg("output-filename")
  use store_encapsulated_pixel_data_flag <- glint.flag(
    store_encapsulated_pixel_data_flag(),
  )
  use named_args, _, flags <- glint.command()

  let input_filename = input_filename(named_args)
  let output_filename = output_filename(named_args)

  let assert Ok(store_encapsulated_pixel_data) =
    store_encapsulated_pixel_data_flag(flags)

  let config = DicomJsonConfig(store_encapsulated_pixel_data:)

  case dcmfx_p10.read_file(input_filename) {
    Ok(data_set) ->
      case dcmfx_json.data_set_to_json(data_set, config) {
        Ok(json_value) ->
          case simplifile.write(output_filename, json_value) {
            Ok(Nil) -> Ok(Nil)
            Error(e) -> {
              p10_error.print(
                p10_error.FileError("writing output file", e),
                "writing file \"" <> output_filename <> "\"",
              )

              Error(Nil)
            }
          }

        Error(json_error.DataError(e)) -> {
          data_error.print(e, "converting data set to JSON")
          Error(Nil)
        }
      }

    Error(e) -> {
      p10_error.print(e, "reading file \"" <> input_filename <> "\"")
      Error(Nil)
    }
  }
}
