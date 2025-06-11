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

/// Converts a list of input filenames passed to a CLI command into a list of
/// input sources.
///
/// This handles recognizing "-" as meaning stdin, and also expands input
/// filenames containing wildcards as glob patterns.
///
pub fn get_input_sources(input_filenames: &[PathBuf]) -> Vec<InputSource> {
  let mut input_sources = vec![];

  for input_filename in input_filenames {
    match input_filename.to_string_lossy().as_ref() {
      "-" => input_sources.push(InputSource::Stdin),

      _ => match glob::glob(&input_filename.to_string_lossy()) {
        Ok(paths) => {
          let paths: Vec<_> = paths.into_iter().collect();

          if paths.is_empty() {
            if !input_filename.is_file() {
              eprintln!("Error: '{}' is not a file", input_filename.display(),);
              std::process::exit(1);
            }

            input_sources.push(InputSource::LocalFile {
              path: PathBuf::from(input_filename),
            });
          } else {
            for path in paths {
              match path {
                Ok(path) => {
                  if path.is_dir() {
                    continue;
                  }

                  input_sources.push(InputSource::LocalFile { path });
                }

                Err(e) => {
                  eprintln!(
                    "Error: Failed globbing input '{}', details: {}",
                    input_filename.display(),
                    e.error()
                  );

                  std::process::exit(1);
                }
              }
            }
          }
        }

        Err(e) => {
          eprintln!(
            "Error: Invalid glob pattern '{}', details: {}",
            input_filename.display(),
            e
          );

          std::process::exit(1);
        }
      },
    }
  }

  input_sources
}
