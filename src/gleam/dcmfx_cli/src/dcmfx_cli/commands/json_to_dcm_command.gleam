import dcmfx_cli/input_source.{type InputSource}
import dcmfx_json
import dcmfx_json/json_error.{type JsonDeserializeError}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_write
import dcmfx_p10/uids
import gleam/bool
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
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
    "The name of the DICOM P10 output file. By default the output DICOM P10 "
    <> "file is the name of the input file with '.dcm' appended.\n\n"
    <> "This argument is not permitted when multiple input files are "
    <> "specified.",
  )
}

fn output_directory_flag() {
  glint.string_flag("output-directory")
  |> glint.flag_help(
    "The directory to write output files into. The names of the output DICOM "
    <> "P10 files will be the name of the input file with '.dcm' appended.",
  )
}

fn implementation_version_name_flag() {
  glint.string_flag("implementation-version-name")
  |> glint.flag_help(
    "Specifies the value of the Implementation Version Name data element in "
    <> "output DICOM P10 files.",
  )
  |> glint.flag_default(uids.dcmfx_implementation_version_name)
}

type JsonToDcmArgs {
  JsonToDcmArgs(
    output_filename: Option(String),
    implementation_version_name: String,
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
  use output_directory <- glint.flag(output_directory_flag())
  use implementation_version_name <- glint.flag(
    implementation_version_name_flag(),
  )
  use _named_args, unnamed_args, flags <- glint.command()

  let input_filenames = unnamed_args
  let output_filename = output_filename(flags) |> option.from_result
  let output_directory = output_directory(flags) |> option.from_result
  let assert Ok(implementation_version_name) =
    implementation_version_name(flags)

  let input_sources = input_source.get_input_sources(input_filenames)

  input_source.validate_output_args(
    input_sources,
    output_filename,
    output_directory,
  )

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

  let args = JsonToDcmArgs(output_filename:, implementation_version_name:)

  input_sources
  |> list.try_each(fn(input_source) {
    let output_filename = case output_filename {
      Some(output_filename) -> output_filename
      None -> input_source.output_path(input_source, ".dcm", output_directory)
    }

    case input_source_to_dcm(input_source, output_filename, args) {
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
  output_filename: String,
  args: JsonToDcmArgs,
) -> Result(Nil, ToDcmError) {
  io.println_error("Writing \"" <> output_filename <> "\" …")

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

  // Read raw DICOM JSON into a data set
  let data_set =
    dcmfx_json.json_to_data_set(json)
    |> result.map_error(ToDcmJsonDeserializeError)
  use data_set <- result.try(data_set)

  let write_config =
    p10_write.P10WriteConfig(
      ..p10_write.default_config(),
      implementation_version_name: args.implementation_version_name,
    )

  // Write P10 data to output file
  dcmfx_p10.write_file(output_filename, data_set, Some(write_config))
  |> result.map_error(ToDcmP10Error)
}
