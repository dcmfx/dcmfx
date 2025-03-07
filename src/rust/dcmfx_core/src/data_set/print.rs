#[cfg(feature = "std")]
use std::io::IsTerminal;

#[cfg(not(feature = "std"))]
use alloc::{
  format,
  string::{String, ToString},
};

use crate::{DataElementTag, DataSet, ValueRepresentation, dictionary, utils};

/// Configurable options used when printing a data set to stdout.
///
#[derive(Clone, Debug, PartialEq)]
pub struct DataSetPrintOptions {
  /// Whether to include styling such as colored text and bold text. This should
  /// only be used when printing to a terminal that supports color.
  ///
  /// By default this is set based on automatically detecting whether the output
  /// stream supports color.
  pub styled: bool,

  /// The maximum output width for the printed data set. Lines that exceed this
  /// length will be truncated with an ellipsis character.
  ///
  /// By default this is set based on automatically detecting the stdout
  /// terminal's width.
  pub max_width: usize,
}

#[cfg(not(target_arch = "wasm32"))]
fn terminal_width() -> Option<usize> {
  if let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size()
  {
    Some(width as usize)
  } else {
    None
  }
}

#[cfg(target_arch = "wasm32")]
fn terminal_width() -> Option<usize> {
  None
}

impl DataSetPrintOptions {
  /// Constructs new data set print options and auto-detects output settings
  /// when possible.
  ///
  pub fn new() -> Self {
    #[cfg(feature = "std")]
    let is_terminal = std::io::stdout().is_terminal();

    #[cfg(not(feature = "std"))]
    let is_terminal = false;

    let color_support =
      supports_color::on(supports_color::Stream::Stdout).is_some();

    Self {
      styled: is_terminal && color_support,
      max_width: terminal_width().unwrap_or(80),
    }
  }

  /// Sets the [`DataSetPrintOptions::styled`] value.
  ///
  pub fn styled(self, styled: bool) -> Self {
    Self { styled, ..self }
  }

  /// Sets the [`DataSetPrintOptions::max_width`] value.
  ///
  pub fn max_width(self, max_width: usize) -> Self {
    Self { max_width, ..self }
  }
}

impl Default for DataSetPrintOptions {
  fn default() -> Self {
    Self::new()
  }
}

/// Recursively prints a data set to stdout using the specified print options.
///
pub fn data_set_to_lines(
  data_set: &DataSet,
  print_options: &DataSetPrintOptions,
  callback: &mut impl FnMut(String),
  indent: usize,
) {
  for (tag, value) in data_set.iter() {
    let (header, header_width) = format_data_element_prefix(
      *tag,
      data_set.tag_name(*tag),
      Some(value.value_representation()),
      value.bytes().map(|bytes| bytes.len()).ok(),
      indent,
      print_options,
    );

    // For sequences, recursively print their items
    if let Ok(items) = value.sequence_items() {
      callback(header);

      for item in items.iter() {
        callback(
          format_data_element_prefix(
            dictionary::ITEM.tag,
            dictionary::ITEM.name,
            None,
            None,
            indent + 1,
            print_options,
          )
          .0,
        );

        data_set_to_lines(item, print_options, callback, indent + 2);

        callback(
          format_data_element_prefix(
            dictionary::ITEM_DELIMITATION_ITEM.tag,
            dictionary::ITEM_DELIMITATION_ITEM.name,
            None,
            None,
            indent + 1,
            print_options,
          )
          .0,
        );
      }

      callback(
        format_data_element_prefix(
          dictionary::SEQUENCE_DELIMITATION_ITEM.tag,
          dictionary::SEQUENCE_DELIMITATION_ITEM.name,
          None,
          None,
          indent,
          print_options,
        )
        .0,
      );
    } else if let Ok(items) = value.encapsulated_pixel_data() {
      callback(header.to_string());

      for item in items {
        let (item_header, item_header_width) = format_data_element_prefix(
          dictionary::ITEM.tag,
          dictionary::ITEM.name,
          None,
          Some(item.len()),
          indent + 1,
          print_options,
        );

        let value_max_width = core::cmp::max(
          print_options.max_width.saturating_sub(item_header_width),
          10,
        );

        callback(format!(
          "{item_header}{}",
          utils::inspect_u8_slice(item, (value_max_width - 2) / 3)
        ));
      }

      callback(
        format_data_element_prefix(
          dictionary::SEQUENCE_DELIMITATION_ITEM.tag,
          dictionary::SEQUENCE_DELIMITATION_ITEM.name,
          None,
          None,
          indent,
          print_options,
        )
        .0,
      );
    } else {
      let value_max_width = core::cmp::max(
        print_options.max_width.saturating_sub(header_width),
        10,
      );

      callback(format!(
        "{header}{}",
        value.to_string(*tag, value_max_width)
      ));
    }
  }
}

/// Formats details for a data element for display on stdout, excluding its
/// value. Returns the string to display along with the number of printable
/// characters.
///
pub fn format_data_element_prefix(
  tag: DataElementTag,
  tag_name: &'static str,
  vr: Option<ValueRepresentation>,
  length: Option<usize>,
  indent: usize,
  print_options: &DataSetPrintOptions,
) -> (String, usize) {
  // Style tag in blue
  let tag = if print_options.styled {
    text_blue(&tag.to_string())
  } else {
    tag.to_string()
  };

  let tag_name_len = tag_name.len();

  // Style tag name in bold
  let tag_name = if print_options.styled {
    text_reset_to_bold(tag_name)
  } else {
    tag_name.to_string()
  };

  let output = if print_options.styled {
    if let Some(vr) = vr {
      // Style VR in green
      let vr = if print_options.styled {
        text_green(&vr.to_string())
      } else {
        vr.to_string()
      };

      format!("{} {} {}", tag, vr, tag_name)
    } else {
      format!("{} {}", tag, tag_name)
    }
  } else if let Some(vr) = vr {
    format!("{} {} {}", tag, vr, tag_name)
  } else {
    format!("{} {}", tag, tag_name)
  };

  let tag_and_vr_width = if vr.is_some() { 15 } else { 12 };

  let has_length = length.is_some();

  let length = if let Some(length) = length {
    format!("[{length:6} bytes] ")
  } else {
    "".to_string()
  };

  let length_width = length.len();

  // Style length in cyan
  let length = if print_options.styled {
    text_cyan_and_reset(&length)
  } else {
    length
  };

  let empty = "";

  let padding = if has_length {
    core::cmp::max(50i64 - (tag_and_vr_width + tag_name_len) as i64, 0) as usize
      + 2
  } else {
    0
  };

  let s = format!(
    "{empty:indent$}{output}{empty:<padding$}{length}",
    indent = indent * 2,
    padding = padding
  );

  let width =
    indent * 2 + tag_and_vr_width + tag_name_len + padding + length_width;

  (s, width)
}

// Simple helpers for coloring and styling text on the terminal. These are used
// instead of a 3rd party crate because the requirements are very simple and the
// functions below are also more efficient due to avoiding unnecessary resets.

fn text_blue(s: &str) -> String {
  format!("\u{001b}[34m{}", s)
}

fn text_cyan_and_reset(s: &str) -> String {
  format!("\u{001b}[36m{}\u{001b}[0m", s)
}

fn text_reset_to_bold(s: &str) -> String {
  format!("\u{001b}[0m\u{001b}[1m{}", s)
}

fn text_green(s: &str) -> String {
  format!("\u{001b}[32m{}", s)
}
