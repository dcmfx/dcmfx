import dcmfx_core/data_set
import dcmfx_core/data_set_print.{type DataSetPrintOptions, DataSetPrintOptions}
import dcmfx_p10
import dcmfx_p10/p10_error.{type P10Error}
import dcmfx_p10/p10_part
import dcmfx_p10/p10_read.{type P10ReadContext}
import dcmfx_p10/transforms/p10_print_transform.{type P10PrintTransform}
import file_streams/file_stream.{type FileStream}
import gleam/io
import gleam/list
import gleam/result
import glint
import snag
import term_size

fn command_help() {
  "Prints the content of a DICOM P10 file"
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
  use input_filename <- glint.named_arg("input-filename")
  use max_width_flag <- glint.flag(max_width_flag())
  use styled_flag <- glint.flag(styled_flag())
  use named_args, _, flags <- glint.command()

  let input_filename = input_filename(named_args)

  let assert Ok(max_width) = max_width_flag(flags)
  let assert Ok(styled) = styled_flag(flags)

  let context = p10_read.new_read_context()

  // Set a small max part size to keep memory usage low. 256 KiB is also plenty
  // of data to preview the content of data element values, even if the max
  // output width is very large.
  let context =
    context
    |> p10_read.with_config(
      p10_read.P10ReadConfig(
        ..p10_read.default_config(),
        max_part_size: 256 * 1024,
      ),
    )

  // Construct print option arguments
  let print_options = DataSetPrintOptions(max_width:, styled:)

  case perform_print(input_filename, context, print_options) {
    Ok(_) -> Ok(Nil)
    Error(e) -> {
      p10_error.print(e, "printing file \"" <> input_filename <> "\"")
      Error(Nil)
    }
  }
}

fn perform_print(
  input_filename: String,
  context: P10ReadContext,
  print_options: DataSetPrintOptions,
) -> Result(Nil, P10Error) {
  let input_stream =
    input_filename
    |> file_stream.open_read
    |> result.map_error(fn(e) {
      p10_error.FileStreamError(
        "Opening input file \"" <> input_filename <> "\"",
        e,
      )
    })
  use input_stream <- result.try(input_stream)

  let p10_print_transform = p10_print_transform.new(print_options)

  let print_parts_result =
    do_perform_print(input_stream, context, p10_print_transform, print_options)

  let close_result =
    file_stream.close(input_stream)
    |> result.map_error(fn(e) {
      p10_error.FileStreamError(
        "Closing input file '" <> input_filename <> "'",
        e,
      )
    })
  use _ <- result.try(close_result)

  print_parts_result
}

fn do_perform_print(
  input_stream: FileStream,
  context: P10ReadContext,
  p10_print_transform: P10PrintTransform,
  print_options: DataSetPrintOptions,
) -> Result(Nil, P10Error) {
  use #(parts, new_context) <- result.try(dcmfx_p10.read_parts_from_stream(
    input_stream,
    context,
  ))

  let p10_print_transform =
    parts
    |> list.fold(p10_print_transform, fn(p10_print_transform, part) {
      case part {
        p10_part.FilePreambleAndDICMPrefix(..) -> p10_print_transform
        p10_part.FileMetaInformation(data_set) -> {
          data_set.print_with_options(data_set, print_options)
          p10_print_transform
        }

        p10_part.End -> p10_print_transform

        _ -> {
          let #(p10_print_transform, s) =
            p10_print_transform.add_part(p10_print_transform, part)

          io.print(s)

          p10_print_transform
        }
      }
    })

  case list.contains(parts, p10_part.End) {
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
