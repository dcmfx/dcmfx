import dcmfx_cli/input_source.{type InputSource}
import dcmfx_json/json_config.{type DicomJsonConfig, DicomJsonConfig}
import dcmfx_json/json_error
import dcmfx_json/transforms/p10_json_transform.{type P10JsonTransform}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read.{type P10ReadContext, P10ReadConfig}
import dcmfx_p10/p10_token
import file_streams/file_stream.{type FileStream}
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import glint

fn command_help() {
  "Converts DICOM P10 files to DICOM JSON files"
}

fn output_filename_flag() {
  glint.string_flag("output-filename")
  |> glint.flag_help(
    "The name of the output DICOM JSON file. This option is only valid "
    <> "when a single input filename is specified.",
  )
}

fn output_directory_flag() {
  glint.string_flag("output-directory")
  |> glint.flag_help(
    "The directory to write output files into. The names of the output DICOM "
    <> "JSON files will be the name of the input file with '.json' appended.",
  )
}

fn pretty_print_flag() {
  glint.bool_flag("pretty")
  |> glint.flag_default(False)
  |> glint.flag_help(
    "Whether to format the DICOM JSON for readability with newlines and "
    <> "indentation",
  )
}

fn store_encapsulated_pixel_data_flag() {
  glint.bool_flag("store-encapsulated-pixel-data")
  |> glint.flag_default(True)
  |> glint.flag_help(
    "Whether to extend DICOM JSON to store encapsulated pixel data as "
    <> "inline binaries. This is a common extension to the DICOM JSON "
    <> "standard.",
  )
}

type ToJsonError {
  ToJsonP10Error(e: P10Error)
  ToJsonSerializeError(e: json_error.JsonSerializeError)
}

pub fn run() {
  use <- glint.command_help(command_help())
  use <- glint.unnamed_args(glint.MinArgs(1))
  use output_filename <- glint.flag(output_filename_flag())
  use output_directory <- glint.flag(output_directory_flag())
  use pretty_print_flag <- glint.flag(pretty_print_flag())
  use store_encapsulated_pixel_data_flag <- glint.flag(
    store_encapsulated_pixel_data_flag(),
  )
  use _named_args, unnamed_args, flags <- glint.command()

  let input_filenames = unnamed_args
  let output_filename = output_filename(flags) |> option.from_result
  let output_directory = output_directory(flags) |> option.from_result
  let assert Ok(pretty_print) = pretty_print_flag(flags)
  let assert Ok(store_encapsulated_pixel_data) =
    store_encapsulated_pixel_data_flag(flags)

  let config = DicomJsonConfig(store_encapsulated_pixel_data:, pretty_print:)

  let input_sources = input_source.get_input_sources(input_filenames)

  input_source.validate_output_args(
    input_sources,
    output_filename,
    output_directory,
  )

  input_sources
  |> list.try_each(fn(input_source) {
    let output_filename = case output_filename {
      Some(output_filename) -> output_filename
      None -> input_source.output_path(input_source, ".json", output_directory)
    }

    case input_source_to_json(input_source, output_filename, config) {
      Ok(Nil) -> Ok(Nil)

      Error(e) -> {
        let task_description = "converting \"" <> input_source <> "\""

        case e {
          ToJsonP10Error(e) -> p10_error.print(e, task_description)
          ToJsonSerializeError(e) ->
            json_error.print_serialize_error(e, task_description)
        }

        Error(Nil)
      }
    }
  })
}

fn input_source_to_json(
  input_source: InputSource,
  output_filename: String,
  config: DicomJsonConfig,
) -> Result(Nil, ToJsonError) {
  // Open input stream
  let input_stream =
    input_source
    |> input_source.open_read_stream
    |> result.map_error(ToJsonP10Error)
  use input_stream <- result.try(input_stream)

  // Open output stream
  io.println_error("Writing \"" <> output_filename <> "\" …")
  let output_stream =
    file_stream.open_write(output_filename)
    |> result.map_error(fn(e) {
      p10_error.FileStreamError("Opening output file", e)
      |> ToJsonP10Error
    })
  use output_stream <- result.try(output_stream)

  // Create P10 read context and set max token size to 256 KiB
  let context =
    p10_read.new_read_context()
    |> p10_read.with_config(
      P10ReadConfig(..p10_read.default_config(), max_token_size: 256 * 1024),
    )

  // Create transform for converting P10 tokens into bytes of JSON
  let json_transform = p10_json_transform.new(config)

  convert_dicom_p10_file_loop(
    input_stream,
    output_stream,
    context,
    json_transform,
  )
}

fn convert_dicom_p10_file_loop(
  input_stream: FileStream,
  output_stream: FileStream,
  context: P10ReadContext,
  json_transform: P10JsonTransform,
) -> Result(Nil, ToJsonError) {
  // Read the next tokens from the input
  case dcmfx_p10.read_tokens_from_stream(input_stream, context) {
    Ok(#(tokens, context)) -> {
      // Write the tokens to the JSON transform, directing the resulting JSON to
      // the output stream
      let json_transform =
        tokens
        |> list.try_fold(json_transform, fn(json_transform, token) {
          case p10_json_transform.add_token(json_transform, token) {
            Ok(#(s, json_transform)) ->
              output_stream
              |> file_stream.write_chars(s)
              |> result.map_error(fn(e) {
                p10_error.FileStreamError("Writing output file", e)
                |> ToJsonP10Error
              })
              |> result.replace(json_transform)

            Error(e) -> Error(ToJsonSerializeError(e))
          }
        })

      case json_transform {
        Ok(json_transform) -> {
          // When the end token has been written the conversion is complete
          case list.contains(tokens, p10_token.End) {
            True ->
              output_stream
              |> file_stream.sync
              |> result.map_error(fn(e) {
                p10_error.FileStreamError("Writing output file", e)
                |> ToJsonP10Error
              })

            False ->
              convert_dicom_p10_file_loop(
                input_stream,
                output_stream,
                context,
                json_transform,
              )
          }
        }

        Error(e) -> Error(e)
      }
    }

    Error(e) -> Error(ToJsonP10Error(e))
  }
}
