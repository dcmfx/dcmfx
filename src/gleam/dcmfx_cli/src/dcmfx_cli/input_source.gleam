import dcmfx_cli/utils
import dcmfx_p10/p10_error.{type P10Error}
import file_streams/file_stream.{type FileStream}
import gleam/io
import gleam/list

/// Defines a single input into a CLI command.
///
pub type InputSource =
  String

pub fn to_string(input_source: InputSource) -> String {
  input_source
}

/// Opens the input source as a read stream.
///
pub fn open_read_stream(
  input_source: InputSource,
) -> Result(FileStream, P10Error) {
  case file_stream.open_read(input_source) {
    Ok(stream) -> Ok(stream)

    Error(error) ->
      Error(p10_error.FileStreamError(when: "Opening file", error:))
  }
}

/// Converts a list of input filenames passed to a CLI command into a list of
/// input sources.
///
pub fn get_input_sources(input_filenames: List(String)) -> List(InputSource) {
  input_filenames
  |> list.fold([], fn(input_sources, input_filename) {
    case input_filename {
      "-" -> {
        io.println_error("Stdin is not supported as an input source")

        utils.exit_with_status(1)

        input_sources
      }

      _ -> [input_filename, ..input_sources]
    }
  })
  |> list.reverse
}
