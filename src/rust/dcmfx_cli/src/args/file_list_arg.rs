use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::InputSource;

pub const ABOUT: &str = "A UTF-8 text file containing a list of input files. \
  Each input file path should be on its own line. White space is trimmed and \
  blank lines are ignored.";

/// Creates an iterator over the paths in the given file list. Each path in the
/// file list file must be on its own line. White space is trimmed and blank
/// lines are ignored.
///
pub fn create_iterator(
  file_list: &Path,
) -> Box<dyn Iterator<Item = InputSource> + Send> {
  let file = match std::fs::File::open(file_list) {
    Ok(file) => file,
    Err(e) => {
      eprintln!(
        "Error: Failed opening file list '{}', details: {}",
        file_list.display(),
        e
      );
      std::process::exit(1);
    }
  };

  let iter = BufReader::new(file).lines().filter_map(|path| match path {
    Ok(path) => {
      let path = path.trim();
      if path.is_empty() {
        None
      } else {
        Some(InputSource::LocalFile { path: path.into() })
      }
    }
    Err(e) => {
      eprintln!("Error: Failed reading file list, details: {}", e);
      std::process::exit(1);
    }
  });

  Box::new(iter)
}
