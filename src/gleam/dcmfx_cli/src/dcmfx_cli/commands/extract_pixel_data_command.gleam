import dcmfx_core/data_set
import dcmfx_core/dictionary
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_pixel_data
import file_streams/file_stream
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
      p10_error.print(e, "reading file \"" <> input_filename <> "\"")
      Error(Nil)
    }
  }
}

fn perform_extract_pixel_data(
  input_filename: String,
  output_prefix: String,
) -> Result(Nil, P10Error) {
  case dcmfx_p10.read_file(input_filename) {
    Ok(data_set) -> {
      let assert Ok(transfer_syntax) =
        data_set.get_string(data_set, dictionary.transfer_syntax_uid.tag)
        |> result.unwrap(transfer_syntax.implicit_vr_little_endian.uid)
        |> transfer_syntax.from_uid

      case dcmfx_pixel_data.get_pixel_data(data_set) {
        Ok(#(_vr, frames)) ->
          case write_frame_data_files(frames, output_prefix, transfer_syntax) {
            Ok(_) -> Ok(Nil)
            Error(e) ->
              Error(p10_error.FileStreamError("Failed writing pixel data", e))
          }

        Error(e) ->
          Error(p10_error.OtherError(
            "Failed getting pixel data",
            string.inspect(e),
          ))
      }
    }

    Error(e) -> Error(e)
  }
}

fn write_frame_data_files(
  frames: List(List(BitArray)),
  output_prefix: String,
  transfer_syntax: TransferSyntax,
) -> Result(Int, FileStreamError) {
  frames
  |> list.try_fold(0, fn(index, frame) {
    let filename =
      output_prefix
      <> "."
      <> string.pad_start(int.to_string(index), 4, "0")
      <> dcmfx_pixel_data.file_extension_for_transfer_syntax(transfer_syntax)

    io.print("Writing file \"" <> filename <> "\" … ")

    use stream <- result.try(file_stream.open_write(filename))
    let write_result =
      list.try_fold(frame, Nil, fn(_, frame_data) {
        file_stream.write_bytes(stream, frame_data)
      })
    let close_result = file_stream.close(stream)

    case result.all([write_result, close_result]) {
      Ok(_) -> {
        io.println("done")
        Ok(index + 1)
      }
      Error(e) -> Error(e)
    }
  })
}
