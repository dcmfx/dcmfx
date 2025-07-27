import dcmfx_cli/input_source.{type InputSource}
import dcmfx_core/data_set
import dcmfx_core/data_set_print.{type DataSetPrintOptions, DataSetPrintOptions}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_p10/p10_read_config
import dcmfx_p10/p10_token
import dcmfx_p10/transforms/p10_print_transform.{type P10PrintTransform}
import file_streams/file_stream.{type FileStream}
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/result
import glint
import snag
import term_size

fn command_help() {
  "Prints the content of DICOM P10 files"
}

fn max_width_flag() {
  glint.int_flag("max-width")
  |> glint.flag_default(term_size.columns() |> result.unwrap(80))
  |> glint.flag_help(
    "The maximum width in characters of the printed output. By default this is "
    <> "set to the width of the active terminal, or 80 characters if the "
    <> "terminal width can't be detected.",
  )
  |> glint.flag_constraint(fn(max_width) {
    case max_width >= 0 && max_width < 10_000 {
      True -> Ok(max_width)
      False -> Error(snag.new("max width must be in the range 0-9999"))
    }
  })
}

fn styled_flag() {
  glint.bool_flag("styled")
  |> glint.flag_default(data_set_print.new_print_options().styled)
  |> glint.flag_help(
    "Whether to print output using color and bold text. By default this is "
    <> "set based on whether there is an active output terminal that supports "
    <> "colored output",
  )
}

pub fn run() {
  use <- glint.command_help(command_help())
  use <- glint.unnamed_args(glint.MinArgs(1))
  use max_width_flag <- glint.flag(max_width_flag())
  use styled_flag <- glint.flag(styled_flag())
  use _named_args, unnamed_args, flags <- glint.command()

  let assert Ok(max_width) = max_width_flag(flags)
  let assert Ok(styled) = styled_flag(flags)

  let input_sources = input_source.get_input_sources(unnamed_args)

  let print_options = DataSetPrintOptions(max_width:, styled:)

  input_sources
  |> list.try_each(fn(input_source) {
    case print_input_source(input_source, print_options) {
      Ok(_) -> Ok(Nil)
      Error(e) -> {
        p10_error.print(
          e,
          "printing \"" <> input_source.to_string(input_source) <> "\"",
        )
        Error(Nil)
      }
    }
  })
}

fn print_input_source(
  input_source: InputSource,
  print_options: DataSetPrintOptions,
) -> Result(Nil, P10Error) {
  use stream <- result.try(input_source.open_read_stream(input_source))

  // Create read context with a small max token size to keep memory usage low.
  // 256 KiB is also plenty of data to preview the content of data element
  // values, even if the max output width is very large.
  let context =
    p10_read_config.new()
    |> p10_read_config.max_token_size(256 * 1024)
    |> Some
    |> p10_read.new_read_context

  let p10_print_transform = p10_print_transform.new(print_options)

  let print_tokens_result =
    do_perform_print(stream, context, p10_print_transform, print_options)

  let _ = file_stream.close(stream)

  print_tokens_result
}

fn do_perform_print(
  input_stream: FileStream,
  context: P10ReadContext,
  p10_print_transform: P10PrintTransform,
  print_options: DataSetPrintOptions,
) -> Result(Nil, P10Error) {
  use #(tokens, new_context) <- result.try(dcmfx_p10.read_tokens_from_stream(
    input_stream,
    context,
    None,
  ))

  let p10_print_transform =
    tokens
    |> list.fold(p10_print_transform, fn(p10_print_transform, token) {
      case token {
        p10_token.FilePreambleAndDICMPrefix(..) -> p10_print_transform
        p10_token.FileMetaInformation(data_set) -> {
          data_set.print_with_options(data_set, print_options)
          p10_print_transform
        }

        p10_token.End -> p10_print_transform

        _ -> {
          let #(s, p10_print_transform) =
            p10_print_transform.add_token(p10_print_transform, token)

          io.print(s)

          p10_print_transform
        }
      }
    })

  case list.contains(tokens, p10_token.End) {
    True -> Ok(Nil)
    False ->
      do_perform_print(
        input_stream,
        new_context,
        p10_print_transform,
        print_options,
      )
  }
}
