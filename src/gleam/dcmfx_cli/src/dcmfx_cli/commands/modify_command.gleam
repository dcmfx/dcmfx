import dcmfx_anonymize
import dcmfx_cli/utils
import dcmfx_core/data_element_tag
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/dictionary
import dcmfx_core/transfer_syntax.{type TransferSyntax}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read
import dcmfx_p10/p10_token
import dcmfx_p10/p10_write.{type P10WriteConfig, P10WriteConfig}
import dcmfx_p10/transforms/p10_filter_transform.{type P10FilterTransform}
import file_streams/file_stream.{type FileStream}
import gleam/bool
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result
import glint
import glint/constraint
import simplifile
import snag

fn command_help() {
  "Reads a DICOM P10 file, applies modifications, and writes out a new DICOM "
  <> "P10 file"
}

fn transfer_syntax_flag() {
  glint.string_flag("transfer-syntax")
  |> glint.flag_help(
    "The transfer syntax for the output DICOM P10 file. This can only convert "
    <> "between the following transfer syntaxes: 'implicit-vr-little-endian', "
    <> "'explicit-vr-little-endian', 'deflated-explicit-vr-little-endian', and "
    <> "'explicit-vr-big-endian'.",
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
    "The data element tags to delete and not include in the output DICOM P10 "
    <> "file. Separate each tag to be removed with a comma. E.g. "
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

pub fn run() {
  use <- glint.command_help(command_help())
  use input_filename <- glint.named_arg("input-filename")
  use output_filename <- glint.named_arg("output-filename")
  use transfer_syntax_flag <- glint.flag(transfer_syntax_flag())
  use zlib_compression_level_flag <- glint.flag(zlib_compression_level_flag())
  use anonymize_flag <- glint.flag(anonymize_flag())
  use delete_tags_flag <- glint.flag(delete_tags_flag())
  use named_args, _, flags <- glint.command()

  let input_filename = input_filename(named_args)
  let output_filename = output_filename(named_args)

  let assert Ok(anonymize) = anonymize_flag(flags)

  // Set the zlib compression level in the write config
  let assert Ok(zlib_compression_level) = zlib_compression_level_flag(flags)
  let write_config =
    P10WriteConfig(zlib_compression_level: zlib_compression_level)

  // Get the list of tags to be deleted
  let assert Ok(tags_to_delete) =
    delete_tags_flag(flags)
    |> result.unwrap([])
    |> list.map(data_element_tag.from_hex_string)
    |> result.all

  // Create a filter transform for anonymization and tag deletion if needed
  let filter_context = case anonymize || !list.is_empty(tags_to_delete) {
    True ->
      p10_filter_transform.new(
        fn(tag, vr, _) {
          { !anonymize || dcmfx_anonymize.filter_tag(tag, vr) }
          && !list.contains(tags_to_delete, tag)
        },
        False,
      )
      |> Some
    False -> None
  }

  let modify_result =
    parse_transfer_syntax_flag(transfer_syntax_flag, flags)
    |> result.then(fn(output_transfer_syntax) {
      streaming_rewrite(
        input_filename,
        output_filename,
        write_config,
        output_transfer_syntax,
        filter_context,
      )
    })

  case modify_result {
    Ok(_) -> Ok(Nil)
    Error(e) -> {
      // Delete any partially written file
      let _ = simplifile.delete(output_filename)

      p10_error.print(e, "modifying file \"" <> input_filename <> "\"")
      Error(Nil)
    }
  }
}

/// Detects and validates the value passed to --transfer-syntax, if present.
///
fn parse_transfer_syntax_flag(
  transfer_syntax_flag: fn(glint.Flags) -> Result(String, a),
  flags: glint.Flags,
) -> Result(Option(TransferSyntax), P10Error) {
  let output_transfer_syntax = transfer_syntax_flag(flags)

  case output_transfer_syntax {
    Ok("implicit-vr-little-endian") ->
      Ok(Some(transfer_syntax.implicit_vr_little_endian))
    Ok("explicit-vr-little-endian") ->
      Ok(Some(transfer_syntax.explicit_vr_little_endian))
    Ok("deflated-explicit-vr-little-endian") ->
      Ok(Some(transfer_syntax.deflated_explicit_vr_little_endian))
    Ok("explicit-vr-big-endian") ->
      Ok(Some(transfer_syntax.explicit_vr_big_endian))

    Ok(ts) ->
      Error(p10_error.OtherError(
        "Unsupported transfer syntax conversion",
        "The transfer syntax '" <> ts <> "' is not recognized",
      ))

    _ -> Ok(None)
  }
}

/// Rewrites by streaming the tokens of the DICOM P10 straight to the output
/// file.
///
fn streaming_rewrite(
  input_filename: String,
  output_filename: String,
  write_config: P10WriteConfig,
  output_transfer_syntax: Option(TransferSyntax),
  filter_context: Option(P10FilterTransform),
) -> Result(Nil, P10Error) {
  // Check that the input and output filenames don't point to the same
  // underlying file. In-place modification isn't supported because of the
  // stream-based implementation.
  use <- bool.guard(
    utils.is_same_file(input_filename, output_filename) == Ok(True),
    Error(p10_error.OtherError(
      "Filename error",
      "Input and output files must be different",
    )),
  )

  // Open input stream
  let input_stream =
    input_filename
    |> file_stream.open_read
    |> result.map_error(p10_error.FileStreamError(
      "Opening input file \"" <> input_filename <> "\"",
      _,
    ))
  use input_stream <- result.try(input_stream)

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
      output_transfer_syntax,
      filter_context,
    )

  // Close input stream
  let input_stream_close_result =
    file_stream.close(input_stream)
    |> result.map_error(p10_error.FileStreamError(
      "Closing input file '" <> input_filename <> "'",
      _,
    ))
  use _ <- result.try(input_stream_close_result)

  // Close output stream
  let output_stream_close_result =
    file_stream.close(output_stream)
    |> result.map_error(p10_error.FileStreamError(
      "Closing output file '" <> output_filename <> "'",
      _,
    ))
  use _ <- result.try(output_stream_close_result)

  rewrite_result
}

fn do_streaming_rewrite(
  input_stream: FileStream,
  output_stream: FileStream,
  p10_read_context: p10_read.P10ReadContext,
  p10_write_context: p10_write.P10WriteContext,
  output_transfer_syntax: Option(TransferSyntax),
  filter_context: Option(P10FilterTransform),
) -> Result(Nil, P10Error) {
  // Read the next P10 tokens from the input stream
  use #(tokens, p10_read_context) <- result.try(
    dcmfx_p10.read_tokens_from_stream(input_stream, p10_read_context),
  )

  // Pass tokens through the filter if one is specified
  let #(tokens, filter_context) = case filter_context {
    Some(filter_context) -> {
      let #(tokens, filter_context) =
        tokens
        |> list.fold(#([], filter_context), fn(in, token) {
          let #(final_tokens, filter_context) = in
          let #(filter_result, filter_context) =
            p10_filter_transform.add_token(filter_context, token)

          let final_tokens = case filter_result {
            True -> list.append(final_tokens, [token])
            False -> final_tokens
          }

          #(final_tokens, filter_context)
        })

      #(tokens, Some(filter_context))
    }

    None -> #(tokens, filter_context)
  }

  // If converting the transfer syntax then update the transfer syntax in the
  // File Meta Information token
  let tokens =
    tokens
    |> list.try_map(fn(token) {
      case output_transfer_syntax, token {
        Some(ts), p10_token.FileMetaInformation(file_meta_information) ->
          file_meta_information
          |> change_transfer_syntax(ts)
          |> result.map(p10_token.FileMetaInformation)

        _, _ -> Ok(token)
      }
    })
  use tokens <- result.try(tokens)

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
    output_transfer_syntax,
    filter_context,
  )
}

/// Adds/updates the *'(0002,0010) TransferSyntaxUID'* data element in the data
/// set. If the current transfer syntax is not able to be converted from then an
/// error is returned.
///
fn change_transfer_syntax(
  data_set: DataSet,
  output_transfer_syntax: TransferSyntax,
) -> Result(DataSet, P10Error) {
  // Read the current transfer syntax, defaulting to 'Implicit VR Little Endian'
  let assert Ok(current_transfer_syntax) =
    data_set.get_string(data_set, dictionary.transfer_syntax_uid.tag)
    |> result.unwrap(transfer_syntax.implicit_vr_little_endian.uid)
    |> transfer_syntax.from_uid

  // The list of transfer syntaxes that can be converted from
  let valid_source_ts = [
    transfer_syntax.implicit_vr_little_endian,
    transfer_syntax.explicit_vr_little_endian,
    transfer_syntax.deflated_explicit_vr_little_endian,
    transfer_syntax.explicit_vr_big_endian,
  ]

  case list.contains(valid_source_ts, current_transfer_syntax) {
    True -> {
      let assert Ok(data_set) =
        data_set
        |> data_set.insert_string_value(dictionary.transfer_syntax_uid, [
          output_transfer_syntax.uid,
        ])

      Ok(data_set)
    }

    False ->
      Error(p10_error.OtherError(
        "Unsupported transfer syntax conversion",
        "The transfer syntax '"
          <> current_transfer_syntax.name
          <> "' is not able to be converted from",
      ))
  }
}
