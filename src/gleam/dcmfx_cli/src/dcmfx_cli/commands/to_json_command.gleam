import dcmfx_json/json_config.{type DicomJsonConfig, DicomJsonConfig}
import dcmfx_json/json_error
import dcmfx_json/transforms/p10_json_transform.{type P10JsonTransform}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_part
import dcmfx_p10/p10_read.{type P10ReadContext, P10ReadConfig}
import file_streams/file_stream.{type FileStream}
import gleam/list
import gleam/result
import glint

fn command_help() {
  "Converts a DICOM P10 file to a DICOM JSON file"
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
  |> glint.flag_default(False)
  |> glint.flag_help(
    "Whether to extend DICOM JSON to store encapsulated pixel data as "
    <> "inline binaries",
  )
}

pub fn run() {
  use <- glint.command_help(command_help())
  use input_filename <- glint.named_arg("input-filename")
  use output_filename <- glint.named_arg("output-filename")
  use pretty_print_flag <- glint.flag(pretty_print_flag())
  use store_encapsulated_pixel_data_flag <- glint.flag(
    store_encapsulated_pixel_data_flag(),
  )
  use named_args, _, flags <- glint.command()

  let input_filename = input_filename(named_args)
  let output_filename = output_filename(named_args)

  let assert Ok(pretty_print) = pretty_print_flag(flags)
  let assert Ok(store_encapsulated_pixel_data) =
    store_encapsulated_pixel_data_flag(flags)

  let config = DicomJsonConfig(store_encapsulated_pixel_data:, pretty_print:)

  case perform_to_json(input_filename, output_filename, config) {
    Ok(Nil) -> Ok(Nil)

    Error(ToJsonSerializeError(e)) -> {
      json_error.print_serialize_error(
        e,
        "converting \"" <> input_filename <> "\" to JSON",
      )
      Error(Nil)
    }

    Error(ToJsonP10Error(e)) -> {
      p10_error.print(e, "converting \"" <> input_filename <> "\" to JSON")
      Error(Nil)
    }
  }
}

type ToJsonError {
  ToJsonSerializeError(e: json_error.JsonSerializeError)
  ToJsonP10Error(e: P10Error)
}

fn perform_to_json(
  input_filename: String,
  output_filename: String,
  config: DicomJsonConfig,
) -> Result(Nil, ToJsonError) {
  // Open input stream
  let input_stream =
    file_stream.open_read(input_filename)
    |> result.map_error(fn(e) {
      p10_error.FileStreamError("Opening input file", e)
      |> ToJsonP10Error
    })
  use input_stream <- result.try(input_stream)

  // Open output stream
  let output_stream =
    file_stream.open_write(output_filename)
    |> result.map_error(fn(e) {
      p10_error.FileStreamError("Opening output file", e)
      |> ToJsonP10Error
    })
  use output_stream <- result.try(output_stream)

  // Create P10 read context and set max part size to 256 KiB
  let context =
    p10_read.new_read_context()
    |> p10_read.with_config(
      P10ReadConfig(..p10_read.default_config(), max_part_size: 256 * 1024),
    )

  // Create transform for converting P10 parts into bytes of JSON
  let json_transform = p10_json_transform.new(config)

  perform_to_json_loop(input_stream, output_stream, context, json_transform)
}

fn perform_to_json_loop(
  input_stream: FileStream,
  output_stream: FileStream,
  context: P10ReadContext,
  json_transform: P10JsonTransform,
) -> Result(Nil, ToJsonError) {
  // Read the next parts from the input
  case dcmfx_p10.read_parts_from_stream(input_stream, context) {
    Ok(#(parts, context)) -> {
      // Write the parts to the JSON transform, directing the resulting JSON to
      // the output stream
      let json_transform =
        parts
        |> list.try_fold(json_transform, fn(json_transform, part) {
          case p10_json_transform.add_part(json_transform, part) {
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
          // When the end part has been written the conversion is complete
          case list.contains(parts, p10_part.End) {
            True ->
              output_stream
              |> file_stream.sync
              |> result.map_error(fn(e) {
                p10_error.FileStreamError("Writing output file", e)
                |> ToJsonP10Error
              })

            False ->
              perform_to_json_loop(
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
