import dcmfx_cli/utils
import dcmfx_p10/p10_error.{type P10Error}
import file_streams/file_stream.{type FileStream}
import filepath
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import simplifile

/// Defines a single input into a CLI command.
///
pub type InputSource =
  String

/// Converts an input source into a human-readable string.
///
pub fn to_string(input_source: InputSource) -> String {
  input_source
}

/// Returns path to the output file for this input source taking into account
/// the specified output suffix and output directory.
///
pub fn output_path(
  input_source: InputSource,
  output_suffix: String,
  output_directory: Option(String),
) -> String {
  case output_directory {
    Some(output_directory) ->
      filepath.join(
        output_directory,
        filepath.base_name(input_source) <> output_suffix,
      )

    None -> input_source <> output_suffix
  }
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

/// Validates the --output-filename and --output-directory arguments for the
/// given input sources.
///
pub fn validate_output_args(
  input_sources: List(InputSource),
  output_filename: Option(String),
  output_directory: Option(String),
) -> Nil {
  // Check that --output-directory is a valid directory
  case output_directory {
    Some(output_directory) -> {
      case simplifile.is_directory(output_directory) {
        Ok(True) -> Nil
        _ -> {
          io.println_error(
            "Error: '" <> output_directory <> "' is not a valid directory",
          )

          utils.exit_with_status(1)
        }
      }
    }

    None -> Nil
  }

  // Check that --output-filename and --output-directory aren't both specified
  case output_filename, output_directory {
    Some(_), Some(_) -> {
      io.println_error(
        "Error: --output-filename and --output-directory can't be specified "
        <> "together",
      )

      utils.exit_with_status(1)
    }

    _, _ -> Nil
  }

  // Check that --output-filename isn't specified when there's more than one
  // input source
  case list.length(input_sources), output_filename {
    length, Some(_) if length > 1 -> {
      io.println_error(
        "Error: --output-filename is not valid when there are multiple input "
        <> "files",
      )

      utils.exit_with_status(1)
    }

    _, _ -> Nil
  }
}
