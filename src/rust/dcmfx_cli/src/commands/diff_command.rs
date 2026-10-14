use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::Args;

use dcmfx::{core::*, p10::*};

use crate::args::input_args::default_transfer_syntax_arg_validate;
use crate::utils::InputSource;

pub const ABOUT: &str =
  "Compares the data sets of two DICOM P10 files and prints their differences";

#[derive(Args)]
pub struct DiffArgs {
  #[arg(help = "The first DICOM P10 file to compare")]
  input_1: PathBuf,

  #[arg(help = "The second DICOM P10 file to compare")]
  input_2: PathBuf,

  #[arg(
    long,
    help_heading = "Input",
    help = "The transfer syntax to use for DICOM P10 data that doesn't specify \
      '(0002,0010) Transfer Syntax UID' in its File Meta Information, or that \
      doesn't have any File Meta Information.\n\
      \n\
      Defaults to '1.2.840.10008.1.2' (Implicit VR Little Endian)",
    value_parser = default_transfer_syntax_arg_validate,
  )]
  default_transfer_syntax: Option<&'static TransferSyntax>,

  #[arg(
    long,
    help_heading = "Output",
    help = "The maximum width in characters of the printed output. By default \
      this is set to the width of the active terminal, or 80 characters if the \
      terminal width can't be detected.",
    value_parser = clap::value_parser!(u32).range(0..10000),
  )]
  max_width: Option<u32>,

  #[arg(
    long,
    help_heading = "Output",
    help = "Whether to print output using color and bold text. By default this \
      is set based on whether there is an active output terminal that supports \
      colored output."
  )]
  styled: Option<bool>,
}

pub async fn run(args: DiffArgs) -> Result<(), ()> {
  let read_config = P10ReadConfig::default().default_transfer_syntax(
    args
      .default_transfer_syntax
      .unwrap_or(&transfer_syntax::IMPLICIT_VR_LITTLE_ENDIAN),
  );

  let data_set_1 = read_data_set(&args.input_1, &read_config).await?;
  let data_set_2 = read_data_set(&args.input_2, &read_config).await?;

  let mut print_options = DataSetPrintOptions::default();
  if let Some(max_width) = args.max_width {
    print_options = print_options.max_width(max_width as usize);
  }
  if let Some(styled) = args.styled {
    print_options = print_options.styled(styled);
  }

  let mut has_differences = false;

  diff_data_sets(&data_set_1, &data_set_2, 0, &print_options, &mut |line| {
    has_differences = true;
    println!("{line}");
  });

  if has_differences { Err(()) } else { Ok(()) }
}

/// Resolves `path` to an input source and reads its full P10 data set
///
async fn read_data_set(
  path: &Path,
  read_config: &P10ReadConfig,
) -> Result<DataSet, ()> {
  let input_source = InputSource::resolve(path).await;

  let mut stream = match input_source.open_read_stream().await {
    Ok(stream) => stream,
    Err(e) => {
      e.print(&format!("opening \"{input_source}\""));
      return Err(());
    }
  };

  match dcmfx::p10::read_stream_async(&mut stream, Some(*read_config)).await {
    Ok(data_set) => Ok(data_set),

    Err((e, _)) => {
      e.print(&format!("reading \"{input_source}\""));
      Err(())
    }
  }
}

/// Recursively diffs two data sets, calling `callback` with one line per
/// difference.
///
/// Elements only in `a` are removed, only in `b` are added, and elements with
/// different values show a removal followed by an addition.
///
fn diff_data_sets(
  a: &DataSet,
  b: &DataSet,
  indent: usize,
  print_options: &DataSetPrintOptions,
  callback: &mut impl FnMut(String),
) {
  let tags: BTreeSet<DataElementTag> =
    a.tags().into_iter().chain(b.tags()).collect();

  for tag in tags {
    match (a.get_value(tag), b.get_value(tag)) {
      (Ok(value_a), Ok(value_b)) => {
        if value_a == value_b {
          continue;
        }

        if let (Ok(items_a), Ok(items_b)) =
          (value_a.sequence_items(), value_b.sequence_items())
        {
          callback(format_container_line(
            ' ',
            tag,
            a.tag_name(tag),
            indent,
            print_options,
          ));

          let item_count = core::cmp::max(items_a.len(), items_b.len());

          for i in 0..item_count {
            match (items_a.get(i), items_b.get(i)) {
              // Item present on both sides - recurse to diff its contents
              (Some(item_a), Some(item_b)) => {
                if item_a == item_b {
                  continue;
                }

                callback(format_item_line(' ', indent + 1, print_options));
                diff_data_sets(
                  item_a,
                  item_b,
                  indent + 2,
                  print_options,
                  callback,
                );
              }

              // Item only in `a` - print it in full as a removal
              (Some(item_a), None) => {
                callback(format_item_line('-', indent + 1, print_options));
                print_whole_data_set(
                  item_a,
                  '-',
                  indent + 2,
                  print_options,
                  callback,
                );
              }

              // Item only in `b` - print it in full as an addition
              (None, Some(item_b)) => {
                callback(format_item_line('+', indent + 1, print_options));
                print_whole_data_set(
                  item_b,
                  '+',
                  indent + 2,
                  print_options,
                  callback,
                );
              }

              // i < item_count, so at least one side has an item here
              (None, None) => unreachable!(),
            }
          }
        } else {
          callback(format_value_line(
            '-',
            tag,
            a,
            value_a,
            indent,
            print_options,
          ));
          callback(format_value_line(
            '+',
            tag,
            b,
            value_b,
            indent,
            print_options,
          ));
        }
      }

      (Ok(value_a), Err(_)) => print_whole_element(
        tag,
        a,
        value_a,
        '-',
        indent,
        print_options,
        callback,
      ),

      (Err(_), Ok(value_b)) => print_whole_element(
        tag,
        b,
        value_b,
        '+',
        indent,
        print_options,
        callback,
      ),

      (Err(_), Err(_)) => unreachable!(),
    }
  }
}

/// Prints a wholly added or removed data element, recursing into any
/// sequence items
///
fn print_whole_element(
  tag: DataElementTag,
  data_set: &DataSet,
  value: &DataElementValue,
  marker: char,
  indent: usize,
  print_options: &DataSetPrintOptions,
  callback: &mut impl FnMut(String),
) {
  if let Ok(items) = value.sequence_items() {
    callback(format_container_line(
      marker,
      tag,
      data_set.tag_name(tag),
      indent,
      print_options,
    ));

    for item in items {
      callback(format_item_line(marker, indent + 1, print_options));
      print_whole_data_set(item, marker, indent + 2, print_options, callback);
    }
  } else {
    callback(format_value_line(
      marker,
      tag,
      data_set,
      value,
      indent,
      print_options,
    ));
  }
}

/// Prints every element of a wholly added or removed sequence item
///
fn print_whole_data_set(
  data_set: &DataSet,
  marker: char,
  indent: usize,
  print_options: &DataSetPrintOptions,
  callback: &mut impl FnMut(String),
) {
  for (tag, value) in data_set.iter() {
    print_whole_element(
      *tag,
      data_set,
      value,
      marker,
      indent,
      print_options,
      callback,
    );
  }
}

/// Formats a diff line for a data element's tag, VR, name, and value
///
fn format_value_line(
  marker: char,
  tag: DataElementTag,
  data_set: &DataSet,
  value: &DataElementValue,
  indent: usize,
  print_options: &DataSetPrintOptions,
) -> String {
  let prefix_width = indent * 2 + 2;
  let value_width =
    core::cmp::max(print_options.max_width.saturating_sub(prefix_width), 10);

  let content = format!(
    "{tag} {vr} {name} {value}",
    vr = value.value_representation(),
    name = data_set.tag_name(tag),
    value = value.to_string(tag, value_width),
  );

  format_marker_line(marker, indent, &content, print_options)
}

/// Formats a diff line for a sequence's tag and name, which has no value
///
fn format_container_line(
  marker: char,
  tag: DataElementTag,
  tag_name: &str,
  indent: usize,
  print_options: &DataSetPrintOptions,
) -> String {
  format_marker_line(
    marker,
    indent,
    &format!("{tag} {tag_name}"),
    print_options,
  )
}

/// Formats a diff line marking a sequence item
///
fn format_item_line(
  marker: char,
  indent: usize,
  print_options: &DataSetPrintOptions,
) -> String {
  format_marker_line(marker, indent, "Item", print_options)
}

/// Prefixes `content` with the marker and indentation, coloring it when
/// styled
///
fn format_marker_line(
  marker: char,
  indent: usize,
  content: &str,
  print_options: &DataSetPrintOptions,
) -> String {
  let line = format!("{marker} {}{content}", "  ".repeat(indent));

  if !print_options.styled {
    return line;
  }

  match marker {
    '+' => format!("\u{1b}[32m{line}\u{1b}[0m"),
    '-' => format!("\u{1b}[31m{line}\u{1b}[0m"),
    _ => line,
  }
}
