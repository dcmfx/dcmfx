import dcmfx_cli/input_source.{type InputSource}
import dcmfx_json
import dcmfx_json/json_error.{type JsonDeserializeError}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import gleam/bool
import gleam/io
import gleam/list
import gleam/option.{type Option, None}
import gleam/result
import gleam/string
import glint
import simplifile

fn command_help() {
  "Converts DICOM JSON files to DICOM P10 files"
}

fn output_filename_flag() {
  glint.string_flag("output-filename")
  |> glint.flag_help(
    "The name of the output DICOM P10 file. This option is only valid when a "
    <> "single input filename is specified.",
  )
}

type ToDcmError {
  ToDcmJsonDeserializeError(JsonDeserializeError)
  ToDcmP10Error(P10Error)
}

pub fn run() {
  use <- glint.command_help(command_help())
  use <- glint.unnamed_args(glint.MinArgs(1))
  use output_filename <- glint.flag(output_filename_flag())
  use _named_args, unnamed_args, flags <- glint.command()

  let input_filenames = unnamed_args
  let output_filename = output_filename(flags) |> option.from_result

  let input_sources = input_source.get_input_sources(input_filenames)

  use <- bool.lazy_guard(
    list.length(input_sources) > 1 && option.is_some(output_filename),
    fn() {
      io.println_error(
        "When there are multiple input files --output-filename must not be "
        <> "specified",
      )
      Error(Nil)
    },
  )

  input_sources
  |> list.try_each(fn(input_source) {
    case input_source_to_dcm(input_source, output_filename) {
      Ok(Nil) -> Ok(Nil)
      Error(e) -> {
        let task_description =
          "converting \"" <> input_source.to_string(input_source) <> "\""

        case e {
          ToDcmJsonDeserializeError(e) ->
            json_error.print_deserialize_error(e, task_description)
          ToDcmP10Error(e) -> p10_error.print(e, task_description)
        }

        Error(Nil)
      }
    }
  })
}

fn input_source_to_dcm(
  input_source: InputSource,
  output_filename: Option(String),
) -> Result(Nil, ToDcmError) {
  // Read the DICOM JSON from the input
  let json =
    simplifile.read(input_source.to_string(input_source))
    |> result.map_error(fn(e) {
      ToDcmP10Error(p10_error.OtherError(
        error_type: "",
        details: string.inspect(e),
      ))
    })
  use json <- result.try(json)

  let output_filename =
    output_filename
    |> option.unwrap(input_source.to_string(input_source) <> ".dcm")

  // Read raw DICOM JSON into a data set
  let data_set =
    dcmfx_json.json_to_data_set(json)
    |> result.map_error(ToDcmJsonDeserializeError)
  use data_set <- result.try(data_set)

  // Write P10 data to output file
  dcmfx_p10.write_file(output_filename, data_set, None)
  |> result.map_error(ToDcmP10Error)
}
