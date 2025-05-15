import dcmfx_anonymize
import dcmfx_cli/input_source.{type InputSource}
import dcmfx_cli/utils.{bool_to_int}
import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read
import dcmfx_p10/p10_write.{type P10WriteConfig, P10WriteConfig}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import dcmfx_p10/uids
import file_streams/file_stream.{type FileStream}
import gleam/bool
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string
import glint
import glint/constraint
import simplifile
import snag

fn command_help() {
  "Modifies the content of DICOM P10 files"
}

fn output_filename_flag() {
  glint.string_flag("output-filename")
  |> glint.flag_help(
    "The name of the DICOM P10 output file.\n\n"
    <> "This argument is not permitted when multiple input files are "
    <> "specified.",
  )
}

fn output_directory_flag() {
  glint.string_flag("output-directory")
  |> glint.flag_help(
    "The directory to write output files into. The names of the output DICOM "
    <> "P10 files will be the same as the input files.",
  )
}

fn in_place_flag() {
  glint.bool_flag("in-place")
  |> glint.flag_default(False)
  |> glint.flag_help(
    "Whether to modify the input files in place, i.e. overwrite them with the "
    <> "newly modified version rather than write it to a new file.\n\n"
    <> "If there is an error during in-place modification of a file then it "
    <> "will not be altered.\n\n"
    <> "WARNING: modification in-place is a potentially irreversible "
    <> "operation.",
  )
}

fn zlib_compression_level_flag() {
  glint.int_flag("zlib-compression-level")
  |> glint.flag_default(6)
  |> glint.flag_help(
    "The zlib compression level to use when outputting to the 'Deflated "
    <> "Explicit VR Little Endian' transfer syntax. The level ranges from 0, "
    <> "meaning no compression, through to 9, which gives the best compression "
    <> "at the cost of speed. Default: 6.",
  )
  |> glint.flag_constraint(fn(level) {
    case level >= 0 && level <= 9 {
      True -> Ok(level)
      False ->
        Error(snag.new("zlib compression level must be in the range 0-9"))
    }
  })
}

fn anonymize_flag() {
  glint.bool_flag("anonymize")
  |> glint.flag_default(False)
  |> glint.flag_help(
    "Whether to anonymize the output DICOM P10 file by removing all patient "
    <> "data elements, other identifying data elements, as well as private "
    <> "data elements. Note that this option does not remove any identifying "
    <> "information that may be baked into the pixel data.",
  )
}

fn delete_tags_flag() {
  glint.strings_flag("delete-tags")
  |> glint.flag_help(
    "Data element tags to delete and not include in the output DICOM P10 file. "
    <> "Separate each tag to be removed with a comma. E.g. "
    <> "--delete-tags=00100010,00100030",
  )
  |> glint.flag_constraint(
    fn(tag) {
      case data_element_tag.from_hex_string(tag) {
        Ok(_) -> Ok(tag)
        Error(Nil) ->
          Error(snag.new(
            "invalid tag '"
            <> tag
            <> "', tags must be exactly 8 hexadecimal digits",
          ))
      }
    }
    |> constraint.each,
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

type ModifyArgs {
  ModifyArgs(
    output_filename: Option(String),
    output_directory: Option(String),
    in_place: Bool,
    zlib_compression_level: Int,
    anonymize: Bool,
    tags_to_delete: List(DataElementTag),
    implementation_version_name: String,
  )
}

pub fn run() {
  use <- glint.command_help(command_help())
  use <- glint.unnamed_args(glint.MinArgs(1))
  use output_filename <- glint.flag(output_filename_flag())
  use output_directory <- glint.flag(output_directory_flag())
  use in_place <- glint.flag(in_place_flag())
  use zlib_compression_level_flag <- glint.flag(zlib_compression_level_flag())
  use anonymize_flag <- glint.flag(anonymize_flag())
  use delete_tags_flag <- glint.flag(delete_tags_flag())
  use implementation_version_name <- glint.flag(
    implementation_version_name_flag(),
  )
  use _named_args, unnamed_args, flags <- glint.command()

  let input_filenames = unnamed_args

  let assert Ok(in_place) = in_place(flags)
  let assert Ok(anonymize) = anonymize_flag(flags)

  // Get the list of tags to be deleted
  let assert Ok(tags_to_delete) =
    delete_tags_flag(flags)
    |> result.unwrap([])
    |> list.map(data_element_tag.from_hex_string)
    |> result.all

  let assert Ok(implementation_version_name) =
    implementation_version_name(flags)

  let args =
    ModifyArgs(
      output_filename: output_filename(flags) |> option.from_result,
      output_directory: output_directory(flags) |> option.from_result,
      in_place:,
      zlib_compression_level: zlib_compression_level_flag(flags)
        |> option.from_result
        |> option.unwrap(6),
      anonymize:,
      tags_to_delete:,
      implementation_version_name:,
    )

  // Check that exactly one output-related arg was specified
  use <- bool.lazy_guard(
    {
      bool_to_int(option.is_some(args.output_filename))
      + bool_to_int(option.is_some(args.output_directory))
      + bool_to_int(args.in_place)
    }
      != 1,
    fn() {
      io.println_error(
        "Exactly one of --output-filename, --output-directory, or --in-place "
        <> "must be specified",
      )
      Error(Nil)
    },
  )

  let input_sources = input_source.get_input_sources(input_filenames)

  input_source.validate_output_args(
    input_sources,
    args.output_filename,
    args.output_directory,
  )

  input_sources
  |> list.try_each(fn(input_source) {
    let output_filename = case args.in_place {
      True -> input_source.to_string(input_source)
      False ->
        case args.output_filename {
          Some(output_filename) -> output_filename
          None ->
            input_source.output_path(input_source, "", args.output_directory)
        }
    }

    case modify_input_source(input_source, output_filename, args) {
      Ok(_) -> Ok(Nil)
      Error(e) -> {
        p10_error.print(
          e,
          "modifying \"" <> input_source.to_string(input_source) <> "\"",
        )
        Error(Nil)
      }
    }
  })
}

fn modify_input_source(
  input_source: InputSource,
  output_filename: String,
  args: ModifyArgs,
) -> Result(Nil, P10Error) {
  case args.in_place {
    True ->
      io.println(
        "Modifying \""
        <> input_source.to_string(input_source)
        <> "\" in place …",
      )
    False ->
      io.println(
        "Modifying \""
        <> input_source.to_string(input_source)
        <> "\" "
        <> " => \""
        <> output_filename
        <> "\" …",
      )
  }

  // Append a random suffix to get a unique name for a temporary output file
  let tmp_output_filename = {
    let random_suffix =
      list.range(0, 15)
      |> list.map(fn(_) {
        let assert Ok(cp) = string.utf_codepoint(97 + int.random(26))
        cp
      })
      |> string.from_utf_codepoints

    output_filename <> "." <> random_suffix <> ".tmp"
  }

  // Create a filter transform for anonymization and tag deletion if needed
  let filter_context = case
    args.anonymize || !list.is_empty(args.tags_to_delete)
  {
    True ->
      p10_filter_transform.new(fn(tag, vr, _length, _location) {
        { !args.anonymize || dcmfx_anonymize.filter_tag(tag, vr) }
        && !list.contains(args.tags_to_delete, tag)
      })
      |> Some
    False -> None
  }

  // Setup write config
  let write_config =
    P10WriteConfig(
      ..p10_write.default_config(),
      implementation_version_name: args.implementation_version_name,
      zlib_compression_level: args.zlib_compression_level,
    )

  let input_stream = input_source.open_read_stream(input_source)
  use input_stream <- result.try(input_stream)

  let rewrite_result =
    streaming_rewrite(
      input_stream,
      tmp_output_filename,
      write_config,
      filter_context,
    )

  let _ = file_stream.close(input_stream)

  case rewrite_result {
    Ok(Nil) ->
      // Rename the temporary file to the desired output filename
      simplifile.rename(tmp_output_filename, output_filename)
      |> result.map_error(fn(e) {
        p10_error.OtherError(
          error_type: "Renaming '"
            <> tmp_output_filename
            <> "' to '"
            <> output_filename
            <> "'",
          details: simplifile.describe_error(e),
        )
      })

    Error(e) -> Error(e)
  }
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  input_stream: FileStream,
  output_filename: String,
  write_config: P10WriteConfig,
  filter_context: Option(P10FilterTransform),
) -> Result(Nil, P10Error) {
  // Open output stream
  let output_stream =
    output_filename
    |> file_stream.open_write
    |> result.map_error(p10_error.FileStreamError(
      "Opening output file '" <> output_filename <> "'",
      _,
    ))
  use output_stream <- result.try(output_stream)

  // Create read and write contexts
  let read_config =
    p10_read.P10ReadConfig(
      ..p10_read.default_config(),
      max_token_size: 256 * 1024,
    )
  let p10_read_context =
    p10_read.new_read_context() |> p10_read.with_config(read_config)
  let p10_write_context =
    p10_write.new_write_context()
    |> p10_write.with_config(write_config)

  // Stream P10 tokens from the input stream to the output stream
  let rewrite_result =
    do_streaming_rewrite(
      input_stream,
      output_stream,
      p10_read_context,
      p10_write_context,
      filter_context,
    )

  // Close input stream
  let input_stream_close_result =
    file_stream.close(input_stream)
    |> result.map_error(p10_error.FileStreamError("Closing input file", _))
  use _ <- result.try(input_stream_close_result)

  // Close output stream
  let output_stream_close_result =
    file_stream.close(output_stream)
    |> result.map_error(p10_error.FileStreamError("Closing output file", _))
  use _ <- result.try(output_stream_close_result)

  rewrite_result
}

fn do_streaming_rewrite(
  input_stream: FileStream,
  output_stream: FileStream,
  p10_read_context: p10_read.P10ReadContext,
  p10_write_context: p10_write.P10WriteContext,
  filter_context: Option(P10FilterTransform),
) -> Result(Nil, P10Error) {
  // Read the next P10 tokens from the input stream
  use #(tokens, p10_read_context) <- result.try(
    dcmfx_p10.read_tokens_from_stream(input_stream, p10_read_context),
  )

  // Pass tokens through the filter if one is specified
  let tokens_and_filter_context = case filter_context {
    Some(filter_context) ->
      tokens
      |> list.try_fold(#([], filter_context), fn(acc, token) {
        let #(final_tokens, filter_context) = acc
        let add_token_result =
          p10_filter_transform.add_token(filter_context, token)
        use #(filter_result, filter_context) <- result.map(add_token_result)

        let final_tokens = case filter_result {
          True -> list.append(final_tokens, [token])
          False -> final_tokens
        }

        #(final_tokens, filter_context)
      })
      |> result.map(fn(acc) { #(acc.0, Some(acc.1)) })

    None -> Ok(#(tokens, filter_context))
  }
  use #(tokens, filter_context) <- result.try(tokens_and_filter_context)

  // Write tokens to the output stream
  use #(ended, p10_write_context) <- result.try(
    dcmfx_p10.write_tokens_to_stream(tokens, output_stream, p10_write_context),
  )

  // Stop when the end token is received
  use <- bool.guard(ended, Ok(Nil))

  // Continue rewriting tokens
  do_streaming_rewrite(
    input_stream,
    output_stream,
    p10_read_context,
    p10_write_context,
    filter_context,
  )
}
