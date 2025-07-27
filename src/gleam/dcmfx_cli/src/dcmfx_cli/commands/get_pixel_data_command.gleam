import dcmfx_cli/input_source.{type InputSource}
import dcmfx_core/data_error
import dcmfx_core/data_set
import dcmfx_p10
import dcmfx_p10/p10_error
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_p10/p10_read_config
import dcmfx_p10/p10_token
import dcmfx_pixel_data
import dcmfx_pixel_data/pixel_data_frame.{type PixelDataFrame}
import dcmfx_pixel_data/transforms/p10_pixel_data_frame_transform.{
  type P10PixelDataFrameTransform, type P10PixelDataFrameTransformError,
}
import file_streams/file_stream.{type FileStream}
import file_streams/file_stream_error.{type FileStreamError}
import filepath
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import gleam/string
import glint

fn command_help() {
  "Extracts pixel data from DICOM P10 files, writing it to image and video "
  <> "files"
}

fn output_directory_flag() {
  glint.string_flag("output-directory")
  |> glint.flag_help(
    "The directory to write output files into. The names of the output files "
    <> "will be the name of the input file suffixed with a 4-digit frame "
    <> "number, and an appropriate file extension.",
  )
}

pub fn run() {
  use <- glint.command_help(command_help())
  use <- glint.unnamed_args(glint.MinArgs(1))
  use output_directory <- glint.flag(output_directory_flag())
  use _named_args, unnamed_args, flags <- glint.command()

  let input_filenames = unnamed_args
  let output_directory = output_directory(flags) |> option.from_result

  let input_sources = input_source.get_input_sources(input_filenames)

  input_source.validate_output_args(input_sources, None, output_directory)

  input_sources
  |> list.try_each(fn(input_source) {
    let output_prefix = case output_directory {
      Some(output_directory) -> filepath.join(output_directory, "/")
      None -> input_source.output_path(input_source, "", output_directory)
    }

    case get_pixel_data_from_input_source(input_source, output_prefix) {
      Ok(Nil) -> Ok(Nil)
      Error(e) -> {
        let task_description =
          "extracting pixel data from \""
          <> input_source.to_string(input_source)
          <> "\""

        case e {
          p10_pixel_data_frame_transform.DataError(e) ->
            data_error.print(e, task_description)
          p10_pixel_data_frame_transform.P10Error(e) ->
            p10_error.print(e, task_description)
        }

        Error(Nil)
      }
    }
  })
}

fn get_pixel_data_from_input_source(
  input_source: InputSource,
  output_prefix: String,
) -> Result(Nil, P10PixelDataFrameTransformError) {
  // Open input stream
  let input_stream =
    input_source.open_read_stream(input_source)
    |> result.map_error(p10_pixel_data_frame_transform.P10Error)
  use input_stream <- result.try(input_stream)

  // Create read context with a small max token size to keep memory usage low
  let read_context =
    p10_read_config.new()
    |> p10_read_config.max_token_size(1024 * 1024)
    |> Some
    |> p10_read.new_read_context

  let pixel_data_frame_transform = p10_pixel_data_frame_transform.new()

  perform_get_pixel_data_loop(
    input_stream,
    read_context,
    pixel_data_frame_transform,
    output_prefix,
    "",
    0,
  )
}

fn perform_get_pixel_data_loop(
  input_stream: FileStream,
  read_context: P10ReadContext,
  pixel_data_frame_transform: P10PixelDataFrameTransform,
  output_prefix: String,
  output_extension: String,
  frame_number: Int,
) -> Result(Nil, P10PixelDataFrameTransformError) {
  // Read the next tokens from the input stream
  case dcmfx_p10.read_tokens_from_stream(input_stream, read_context, None) {
    Ok(#(tokens, read_context)) -> {
      let context = #(
        output_extension,
        pixel_data_frame_transform,
        frame_number,
        False,
      )

      let context =
        list.try_fold(tokens, context, fn(context, token) {
          let #(output_extension, pixel_data_frame_filter, frame_number, ended) =
            context

          // Determine the output extension from the transfer syntax
          let output_extension = case token {
            p10_token.FileMetaInformation(data_set:) ->
              data_set
              |> data_set.get_transfer_syntax
              |> result.map(dcmfx_pixel_data.file_extension_for_transfer_syntax)
              |> result.unwrap(output_extension)
            _ -> output_extension
          }

          // Pass token through the pixel data filter
          case
            p10_pixel_data_frame_transform.add_token(
              pixel_data_frame_filter,
              token,
            )
          {
            Ok(#(frames, pixel_data_frame_filter)) -> {
              // Write frames
              let frame_number =
                frames
                |> list.try_fold(frame_number, fn(frame_number, frame) {
                  let filename =
                    output_prefix
                    <> "."
                    <> string.pad_start(int.to_string(frame_number), 4, "0")
                    <> output_extension

                  write_frame(filename, frame)
                  |> result.replace(frame_number + 1)
                })
                |> result.map_error(fn(e) {
                  p10_pixel_data_frame_transform.P10Error(
                    p10_error.FileStreamError("Writing pixel data frame", e),
                  )
                })

              case frame_number {
                Ok(frame_number) -> {
                  let ended = ended || token == p10_token.End
                  Ok(#(
                    output_extension,
                    pixel_data_frame_filter,
                    frame_number,
                    ended,
                  ))
                }
                Error(e) -> Error(e)
              }
            }

            Error(e) -> Error(e)
          }
        })

      case context {
        Ok(#(output_extension, pixel_data_frame_transform, frame_number, False)) ->
          perform_get_pixel_data_loop(
            input_stream,
            read_context,
            pixel_data_frame_transform,
            output_prefix,
            output_extension,
            frame_number,
          )

        Ok(#(_, _, _, True)) -> Ok(Nil)

        Error(e) -> Error(e)
      }
    }

    Error(e) -> Error(p10_pixel_data_frame_transform.P10Error(e))
  }
}

/// Writes the data for a single frame of pixel data to a file.
///
fn write_frame(
  filename: String,
  frame: PixelDataFrame,
) -> Result(Nil, FileStreamError) {
  io.println("Writing \"" <> filename <> "\" …")

  use stream <- result.try(file_stream.open_write(filename))

  let fragments = case pixel_data_frame.bit_offset(frame) {
    0 -> pixel_data_frame.chunks(frame)
    _ -> [pixel_data_frame.to_bytes(frame)]
  }

  let write_result =
    fragments
    |> list.try_fold(Nil, fn(_, fragment) {
      file_stream.write_bytes(stream, fragment)
    })

  let close_result = file_stream.close(stream)

  [write_result, close_result]
  |> result.all
  |> result.replace(Nil)
}
