use std::path::PathBuf;

use dcmfx::p10::P10Error;

/// Defines a single input into a CLI command, which can either be the `stdin`
/// stream or a file on the local file system.
///
#[derive(Clone, Debug, PartialEq)]
pub enum InputSource {
  Stdin,
  LocalFile { path: PathBuf },
}

impl core::fmt::Display for InputSource {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    match self {
      InputSource::Stdin => write!(f, "<stdin>"),
      InputSource::LocalFile { path } => write!(f, "{}", path.display()),
    }
  }
}

impl InputSource {
  /// Returns the input source as a [`PathBuf`].
  ///
  pub fn path(&self) -> PathBuf {
    match self {
      InputSource::Stdin => PathBuf::from("stdin"),
      InputSource::LocalFile { path } => path.clone(),
    }
  }

  /// Returns whether the input source is valid DICOM P10 data.
  ///
  pub fn is_dicom_p10(&self) -> bool {
    match self {
      InputSource::Stdin => true,
      InputSource::LocalFile { path } => dcmfx::p10::is_valid_file(path),
    }
  }

  /// Returns path to the output file for this input source taking into account
  /// the specified output suffix and output directory.
  ///
  pub fn output_path(
    &self,
    output_suffix: &str,
    output_directory: &Option<PathBuf>,
  ) -> PathBuf {
    let mut path = self.path();

    if let Some(output_directory) = output_directory {
      output_directory.join(format!(
        "{}{}",
        path.file_name().unwrap().to_string_lossy(),
        output_suffix
      ))
    } else {
      if let Some(file_name) = path.file_name() {
        let new_file_name =
          format!("{}{output_suffix}", file_name.to_string_lossy());
        path.set_file_name(new_file_name);
      }

      path
    }
  }

  /// Opens the input source as a read stream.
  ///
  pub fn open_read_stream(&self) -> Result<Box<dyn std::io::Read>, P10Error> {
    match self {
      InputSource::Stdin => Ok(Box::new(std::io::stdin())),

      InputSource::LocalFile { path } => match std::fs::File::open(path) {
        Ok(file) => Ok(Box::new(file)),

        Err(e) => Err(P10Error::FileError {
          when: "Opening file".to_string(),
          details: e.to_string(),
        }),
      },
    }
  }
}

/// Converts a list of input filenames and an optional input file containing the
/// path of more input files into a lazy iterator over all input sources.
///
/// This handles recognizing "-" as meaning stdin, and expands input filenames
/// containing wildcards as glob patterns.
///
pub fn create_iterator(
  input_filenames: &mut Vec<PathBuf>,
  file_list: &Option<PathBuf>,
) -> Box<dyn Iterator<Item = InputSource> + Send> {
  if input_filenames.is_empty() && file_list.is_none() {
    eprintln!(
      "Error: No inputs specified. Pass --help for usage instructions."
    );
    std::process::exit(1);
  }

  // Create iterator over the input filenames, expanding them if they are glob
  // patterns
  let iter =
    std::mem::take(input_filenames)
      .into_iter()
      .flat_map(|input_filename| {
        let input_filename_str = input_filename.to_string_lossy();

        // Handle stdin
        if input_filename_str == "-" {
          return Box::new(std::iter::once(InputSource::Stdin))
            as Box<dyn Iterator<Item = InputSource> + Send>;
        }

        // Attempt to expand each input filename as a glob pattern
        match glob::glob(&input_filename_str) {
          Ok(paths) => {
            let expanded = paths.filter_map(move |path| match path {
              Ok(path) => {
                if path.is_file() {
                  Some(InputSource::LocalFile { path })
                } else {
                  None
                }
              }
              Err(e) => {
                eprintln!(
                  "Error: Failed globbing '{}', details: {}",
                  input_filename.display(),
                  e
                );
                std::process::exit(1);
              }
            });

            Box::new(expanded)
          }

          Err(e) => {
            eprintln!(
              "Error: Invalid glob pattern '{}', details: {}",
              input_filename.display(),
              e
            );
            std::process::exit(1);
          }
        }
      });

  // If there is a file list then iterate its contents as well
  if let Some(file_list) = file_list {
    Box::new(iter.chain(crate::args::file_list_arg::create_iterator(file_list)))
  } else {
    Box::new(iter)
  }
}
