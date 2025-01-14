import dcmfx_core/data_element_tag.{type DataElementTag}
import dcmfx_core/internal/utils
import dcmfx_core/value_representation.{type ValueRepresentation}
import envoy
import gleam/int
import gleam/option.{type Option, None, Some}
import gleam/result
import gleam/string
import term_size

/// Configurable options used when printing a data set to stdout.
///
pub type DataSetPrintOptions {
  DataSetPrintOptions(
    /// Whether to include styling such as colored text and bold text. This
    /// should only be used when printing to a terminal that supports color.
    ///
    /// By default this is set based on automatically detecting whether the
    /// output terminal supports color.
    styled: Bool,
    /// The maximum output width for the printed data set. Lines that exceed
    /// this length will be truncated with an ellipsis character.
    ///
    /// By default this is set based on automatically detecting the stdout
    /// terminal's width.
    max_width: Int,
  )
}

/// Constructs new data set print options and auto-detects output settings when
/// possible.
///
pub fn new_print_options() -> DataSetPrintOptions {
  let term = envoy.get("TERM")
  let colorterm = envoy.get("COLORTERM")
  let no_color = envoy.get("NO_COLOR")

  let styled =
    { result.is_ok(colorterm) || result.is_ok(term) && term != Ok("dumb") }
    && { no_color != Ok("1") }

  let max_width = term_size.columns() |> result.unwrap(80)

  DataSetPrintOptions(styled:, max_width:)
}

/// Formats details for a data element for display on stdout, excluding its
/// value. Returns the string to display along with the number of printable
/// characters.
///
pub fn format_data_element_prefix(
  tag: DataElementTag,
  tag_name: String,
  vr: Option(ValueRepresentation),
  length: Option(Int),
  indent: Int,
  print_options: DataSetPrintOptions,
) -> #(String, Int) {
  // Style tag in blue
  let tag = case print_options.styled {
    True -> tag |> data_element_tag.to_string |> text_blue
    False -> tag |> data_element_tag.to_string
  }

  let tag_name_len = utils.string_fast_length(tag_name)

  // Style tag name in bold
  let tag_name = case print_options.styled {
    True -> text_reset_to_bold(tag_name)
    False -> tag_name
  }

  let output = case print_options.styled {
    True ->
      case vr {
        Some(vr) -> {
          // Style VR in green
          let vr = case print_options.styled {
            True -> vr |> value_representation.to_string |> text_green
            False -> vr |> value_representation.to_string
          }

          tag <> " " <> vr <> " " <> tag_name
        }

        None -> tag <> " " <> tag_name
      }

    False ->
      case vr {
        Some(vr) ->
          tag <> " " <> value_representation.to_string(vr) <> " " <> tag_name
        None -> tag <> " " <> tag_name
      }
  }

  let tag_and_vr_width = case vr {
    Some(_) -> 15
    None -> 12
  }

  let has_length = length != None

  let length = case length {
    Some(length) ->
      "["
      <> { length |> int.to_string |> utils.pad_start(6, " ") }
      <> " bytes] "
    None -> ""
  }

  let length_width = length |> utils.string_fast_length

  // Style length in cyan
  let length = case print_options.styled {
    True -> text_cyan_and_reset(length)
    False -> length
  }

  let padding = case has_length {
    True -> int.max(50 - { tag_and_vr_width + tag_name_len }, 0) + 2
    False -> 0
  }

  let s =
    string.repeat(" ", indent * 2)
    <> output
    <> string.repeat(" ", padding)
    <> length

  let width =
    indent * 2 + tag_and_vr_width + tag_name_len + padding + length_width

  #(s, width)
}

// Simple helpers for coloring and styling text on the terminal

fn text_reset() -> String {
  "\u{001b}[0m"
}

fn text_blue(s: String) {
  "\u{001b}[34m" <> s
}

fn text_cyan_and_reset(s: String) {
  "\u{001b}[36m" <> s <> text_reset()
}

fn text_reset_to_bold(s: String) {
  text_reset() <> "\u{001b}[1m" <> s
}

fn text_green(s: String) {
  "\u{001b}[32m" <> s
}
