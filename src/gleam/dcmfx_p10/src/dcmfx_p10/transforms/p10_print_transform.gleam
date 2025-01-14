import dcmfx_core/data_element_tag.{type DataElementTag, DataElementTag}
import dcmfx_core/data_element_value
import dcmfx_core/data_set.{type DataSet}
import dcmfx_core/data_set_print.{type DataSetPrintOptions}
import dcmfx_core/dictionary
import dcmfx_core/value_representation
import dcmfx_p10/p10_part.{type P10Part}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result

/// Transform that converts a stream of DICOM P10 parts into printable text
/// that describes the structure and content of the contained DICOM data.
///
/// This is used for printing data sets on the command line, and the output can
/// be styled via `DataSetPrintOptions`.
///
pub type P10PrintTransform {
  P10PrintTransform(
    print_options: DataSetPrintOptions,
    indent: Int,
    current_data_element: DataElementTag,
    ignore_data_element_value_bytes: Bool,
    value_max_width: Int,
    // Track private creator data elements so that private tags can be printed
    // with the correct names where possible
    private_creators: List(DataSet),
    last_data_element_private_creator_tag: Option(DataElementTag),
  )
}

/// Constructs a new DICOM P10 print transform with the specified print options.
///
pub fn new(print_options: DataSetPrintOptions) -> P10PrintTransform {
  P10PrintTransform(
    print_options:,
    indent: 0,
    current_data_element: DataElementTag(0, 0),
    ignore_data_element_value_bytes: False,
    value_max_width: 0,
    private_creators: [data_set.new()],
    last_data_element_private_creator_tag: None,
  )
}

/// Adds the next DICOM P10 part to be printed and returns the next piece of
/// text output to be displayed.
///
pub fn add_part(
  context: P10PrintTransform,
  part: P10Part,
) -> #(String, P10PrintTransform) {
  case part {
    p10_part.FileMetaInformation(data_set) -> #(
      data_set.to_lines(data_set, context.print_options, "", fn(s, line) {
        s <> line <> "\n"
      }),
      context,
    )

    p10_part.DataElementHeader(tag, vr, length) -> {
      let assert Ok(private_creators) = list.first(context.private_creators)

      let #(s, width) =
        data_set_print.format_data_element_prefix(
          tag,
          data_set.tag_name(private_creators, tag),
          Some(vr),
          Some(length),
          context.indent,
          context.print_options,
        )

      // Calculate the width remaining for previewing the value
      let value_max_width = int.max(context.print_options.max_width - width, 10)

      // Use the next value bytes part to print a preview of the data element's
      // value
      let ignore_data_element_value_bytes = False

      // If this is a private creator tag then its value will be stored so that
      // well-known private tag names can be printed
      let last_data_element_private_creator_tag = case
        vr == value_representation.LongString
        && data_element_tag.is_private_creator(tag)
      {
        True -> Some(tag)
        False -> None
      }

      let new_context =
        P10PrintTransform(
          ..context,
          current_data_element: tag,
          value_max_width:,
          ignore_data_element_value_bytes:,
          last_data_element_private_creator_tag:,
        )

      #(s, new_context)
    }

    p10_part.DataElementValueBytes(vr, data, ..)
      if !context.ignore_data_element_value_bytes
    -> {
      let value = data_element_value.new_binary_unchecked(vr, data)

      // Ignore any further value bytes parts now that the value has been
      // printed
      let ignore_data_element_value_bytes = True

      // Store private creator name data elements
      let private_creators = case
        context.last_data_element_private_creator_tag,
        context.private_creators
      {
        Some(tag), [private_creators, ..rest] -> [
          data_set.insert(
            private_creators,
            tag,
            data_element_value.new_binary_unchecked(
              value_representation.LongString,
              data,
            ),
          ),
          ..rest
        ]

        _, _ -> context.private_creators
      }

      let s =
        data_element_value.to_string(
          value,
          context.current_data_element,
          context.value_max_width,
        )
        <> "\n"

      let new_context =
        P10PrintTransform(
          ..context,
          ignore_data_element_value_bytes:,
          private_creators:,
        )

      #(s, new_context)
    }

    p10_part.SequenceStart(tag, vr) -> {
      let assert Ok(private_creators) = list.first(context.private_creators)

      let s =
        data_set_print.format_data_element_prefix(
          tag,
          data_set.tag_name(private_creators, tag),
          Some(vr),
          None,
          context.indent,
          context.print_options,
        ).0

      let new_context = P10PrintTransform(..context, indent: context.indent + 1)

      #(s <> "\n", new_context)
    }

    p10_part.SequenceDelimiter -> {
      let s =
        data_set_print.format_data_element_prefix(
          dictionary.sequence_delimitation_item.tag,
          dictionary.sequence_delimitation_item.name,
          None,
          None,
          context.indent - 1,
          context.print_options,
        ).0

      let new_context = P10PrintTransform(..context, indent: context.indent - 1)

      #(s <> "\n", new_context)
    }

    p10_part.SequenceItemStart -> {
      let s =
        data_set_print.format_data_element_prefix(
          dictionary.item.tag,
          dictionary.item.name,
          None,
          None,
          context.indent,
          context.print_options,
        ).0

      let new_context =
        P10PrintTransform(
          ..context,
          indent: context.indent + 1,
          private_creators: [data_set.new(), ..context.private_creators],
        )

      #(s <> "\n", new_context)
    }

    p10_part.SequenceItemDelimiter -> {
      let s =
        data_set_print.format_data_element_prefix(
          dictionary.item_delimitation_item.tag,
          dictionary.item_delimitation_item.name,
          None,
          None,
          context.indent - 1,
          context.print_options,
        ).0

      let new_context =
        P10PrintTransform(
          ..context,
          indent: context.indent - 1,
          private_creators: list.rest(context.private_creators)
            |> result.unwrap(context.private_creators),
        )

      #(s <> "\n", new_context)
    }

    p10_part.PixelDataItem(length) -> {
      let #(s, width) =
        data_set_print.format_data_element_prefix(
          dictionary.item.tag,
          dictionary.item.name,
          None,
          Some(length),
          context.indent,
          context.print_options,
        )

      // Calculate the width remaining for previewing the value
      let value_max_width = int.max(context.print_options.max_width - width, 10)

      // Use the next value bytes part to print a preview of the pixel data
      // item's value
      let ignore_data_element_value_bytes = False

      let new_context =
        P10PrintTransform(
          ..context,
          value_max_width:,
          ignore_data_element_value_bytes:,
        )

      #(s, new_context)
    }

    _ -> #("", context)
  }
}
