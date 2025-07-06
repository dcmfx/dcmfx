use std::{
  fs::File,
  io::Write,
  path::{Path, PathBuf},
};

use dcmfx::p10::P10Error;

/// Creates a Rayon thread pool with the specified number of threads.
///
pub fn create_thread_pool(threads: usize) -> rayon::ThreadPool {
  rayon::ThreadPoolBuilder::new()
    .num_threads(threads)
    .build()
    .unwrap()
}

/// Opens an output stream for the given path, first checking whether it exists
/// and prompting the user about overwriting it if necessary. This prompt isn't
/// presented to the user if `overwrite` is true.
///
/// The path "-" is interpreted as writing to stdout.
///
pub fn open_output_stream(
  path: &PathBuf,
  display_path: Option<&PathBuf>,
  overwrite: bool,
) -> Result<Box<dyn std::io::Write>, P10Error> {
  if *path == PathBuf::from("-") {
    Ok(Box::new(std::io::stdout()))
  } else {
    if let Some(display_path) = display_path {
      println!("Writing \"{}\" …", display_path.display());
    }

    if !overwrite {
      error_if_exists(path);
    }

    match File::create(path) {
      Ok(file) => Ok(Box::new(file)),

      Err(e) => Err(P10Error::FileError {
        when: "Opening file".to_string(),
        details: e.to_string(),
      }),
    }
  }
}

/// Prints an error and exits the process if the specified file exists.
///
pub fn error_if_exists(path: &Path) {
  if !path.exists() {
    return;
  }

  let _ = std::io::stdout().flush();

  eprintln!(
    "Error: Output file \"{}\" already exists. Specify --overwrite to
     automatically overwrite existing files",
    path.display()
  );
  std::process::exit(1);
}

/// Appends a suffix to a path.
///
pub fn path_append(mut path: PathBuf, suffix: &str) -> PathBuf {
  path.set_file_name(format!(
    "{}{}",
    path
      .file_name()
      .unwrap_or(std::ffi::OsStr::new(""))
      .to_string_lossy(),
    suffix
  ));

  path
}

/// Renames a temporary file to an output filename when [`Self::commit()`] is
/// called, otherwise deletes the temporary file on drop.
///
pub struct TempFileRenamer {
  temp_filename: PathBuf,
  output_filename: PathBuf,
  committed: bool,
}

impl TempFileRenamer {
  pub fn new(temp_filename: PathBuf, output_filename: PathBuf) -> Self {
    Self {
      temp_filename,
      output_filename,
      committed: false,
    }
  }

  pub fn commit(&mut self) -> Result<(), (String, String)> {
    self.committed = true;

    std::fs::rename(&self.temp_filename, &self.output_filename).map_err(|e| {
      (
        format!(
          "Renaming '{}' to '{}'",
          self.temp_filename.display(),
          self.output_filename.display()
        ),
        e.to_string(),
      )
    })
  }
}

impl Drop for TempFileRenamer {
  fn drop(&mut self) {
    if !self.committed {
      let _ = std::fs::remove_file(&self.temp_filename);
    }
  }
}
