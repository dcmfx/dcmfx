import dcmfx_core/data_error.{type DataError}
import dcmfx_core/data_set
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_part
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_pixel_data
import dcmfx_pixel_data/pixel_data_filter.{type PixelDataFilter}
import dcmfx_pixel_data/pixel_data_frame.{type PixelDataFrame}
import file_streams/file_stream.{type FileStream}
import file_streams/file_stream_error.{type FileStreamError}
import gleam/int
import gleam/io
import gleam/list
import gleam/result
import gleam/string
import glint

fn command_help() {
  "Extracts the pixel data from a DICOM P10 file and writes each frame "
  <> "to a separate image file"
}

fn output_prefix_flag() {
  glint.string_flag("output-prefix")
  |> glint.flag_help(
    "The prefix for output image files. It is suffixed with a 4-digit frame "
    <> "number. By default, the output prefix is the input filename.",
  )
}

pub type ExtractPixelDataError {
  ExtractPixelDataDataError(e: DataError)
  ExtractPixelDataP10Error(e: P10Error)
}

pub fn run() {
  use <- glint.command_help(command_help())
  use input_filename <- glint.named_arg("input-filename")
  use output_prefix_flag <- glint.flag(output_prefix_flag())
  use named_args, _, flags <- glint.command()

  let input_filename = input_filename(named_args)
  let output_prefix = output_prefix_flag(flags) |> result.unwrap(input_filename)

  case perform_extract_pixel_data(input_filename, output_prefix) {
    Ok(Nil) -> Ok(Nil)
    Error(e) -> {
      let task = "reading file \"" <> input_filename <> "\""

      case e {
        ExtractPixelDataDataError(e) -> data_error.print(e, task)
        ExtractPixelDataP10Error(e) -> p10_error.print(e, task)
      }

      Error(Nil)
    }
  }
}

fn perform_extract_pixel_data(
  input_filename: String,
  output_prefix: String,
) -> Result(Nil, ExtractPixelDataError) {
  // Open input stream
  let input_stream =
    file_stream.open_read(input_filename)
    |> result.map_error(fn(e) {
      ExtractPixelDataP10Error(p10_error.FileStreamError("Opening file", e))
    })
  use input_stream <- result.try(input_stream)

  // Create read context
  let read_context =
    p10_read.new_read_context()
    |> p10_read.with_config(
      p10_read.P10ReadConfig(
        ..p10_read.default_config(),
        max_part_size: 1024 * 1024,
      ),
    )

  do_perform_extract_pixel_data(
    input_stream,
    read_context,
    pixel_data_filter.new(),
    output_prefix,
    "",
    0,
  )
}

fn do_perform_extract_pixel_data(
  input_stream: FileStream,
  read_context: P10ReadContext,
  pixel_data_filter: PixelDataFilter,
  output_prefix: String,
  output_extension: String,
  frame_number: Int,
) -> Result(Nil, ExtractPixelDataError) {
  // Read the next parts from the input stream
  case dcmfx_p10.read_parts_from_stream(input_stream, read_context) {
    Ok(#(parts, read_context)) -> {
      let context = #(output_extension, pixel_data_filter, frame_number, False)

      let context =
        list.try_fold(parts, context, fn(context, part) {
          let #(output_extension, pixel_data_filter, frame_number, ended) =
            context

          // Update output extension when the File Meta Information part is
          // received
          let output_extension = case part {
            p10_part.FileMetaInformation(data_set:) ->
              data_set
              |> data_set.get_transfer_syntax
              |> result.map(dcmfx_pixel_data.file_extension_for_transfer_syntax)
              |> result.unwrap(output_extension)
            _ -> output_extension
          }

          // Pass part through the pixel data filter
          case
            pixel_data_filter.add_part(pixel_data_filter, part)
            |> result.map_error(ExtractPixelDataDataError)
          {
            Ok(#(frames, pixel_data_filter)) -> {
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
                  ExtractPixelDataP10Error(p10_error.FileStreamError(
                    "Writing pixel data frame",
                    e,
                  ))
                })

              case frame_number {
                Ok(frame_number) -> {
                  let ended = ended || part == p10_part.End
                  Ok(#(output_extension, pixel_data_filter, frame_number, ended))
                }
                Error(e) -> Error(e)
              }
            }

            Error(e) -> Error(e)
          }
        })

      case context {
        Ok(#(output_extension, pixel_data_filter, frame_number, False)) ->
          do_perform_extract_pixel_data(
            input_stream,
            read_context,
            pixel_data_filter,
            output_prefix,
            output_extension,
            frame_number,
          )

        Ok(#(_, _, _, True)) -> Ok(Nil)

        Error(e) -> Error(e)
      }
    }

    Error(e) -> Error(ExtractPixelDataP10Error(e))
  }
}

/// Writes the data for a single frame of pixel data to a file.
///
fn write_frame(
  filename: String,
  frame: PixelDataFrame,
) -> Result(Nil, FileStreamError) {
  io.print(
    "Writing \""
    <> filename
    <> "\", size: "
    <> int.to_string(pixel_data_frame.length(frame))
    <> " bytes … ",
  )

  use stream <- result.try(file_stream.open_write(filename))

  let write_result =
    frame
    |> pixel_data_frame.fragments
    |> list.try_fold(Nil, fn(_, fragment) {
      file_stream.write_bytes(stream, fragment)
    })

  let close_result = file_stream.close(stream)

  [write_result, close_result]
  |> result.all
  |> result.map(fn(_) { io.println("done") })
}
