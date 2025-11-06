use std::{
  path::{Path, PathBuf},
  sync::{Arc, LazyLock},
};

use futures::stream::StreamExt;
use tokio::sync::Mutex;

use dcmfx::p10::P10Error;

use crate::args::input_args::InputSource;

/// Runs tasks concurrently up to the specified task count, passing each item
/// from the given input source iterator to the provided async body function.
///
/// Returns an error as soon as any of the tasks returns an error.
///
pub async fn run_tasks<I, E>(
  task_count: usize,
  input_sources: I,
  body_func: impl AsyncFn(InputSource) -> Result<(), E>,
) -> Result<(), E>
where
  I: Iterator<Item = InputSource>,
{
  let mut task_stream = futures::stream::iter(input_sources)
    .map(async |i| body_func(i).await)
    .buffer_unordered(task_count.max(1));

  while let Some(result) = task_stream.next().await {
    match result {
      Ok(()) => (),
      Err(e) => return Err(e),
    }
  }

  Ok(())
}

/// Shared stdout write stream used for synchronization across threads.
///
pub static GLOBAL_STDOUT: LazyLock<
  Arc<Mutex<Box<dyn dcmfx::p10::IoAsyncWrite + Send>>>,
> = LazyLock::new(|| Arc::new(Mutex::new(Box::new(tokio::io::stdout()))));

/// Opens an output stream for the given path, first checking whether it exists
/// and prompting the user about overwriting it if necessary. This prompt isn't
/// presented to the user if `overwrite` is true.
///
/// The path "-" is interpreted as writing to stdout.
///
pub async fn open_output_stream(
  path: &PathBuf,
  display_path: Option<&PathBuf>,
  overwrite: bool,
) -> Result<Arc<Mutex<Box<dyn dcmfx::p10::IoAsyncWrite + Send>>>, P10Error> {
  if path.to_string_lossy() == "-" {
    Ok(GLOBAL_STDOUT.clone())
  } else {
    if let Some(display_path) = display_path {
      println!("Writing \"{}\" …", display_path.display());
    }

    if !overwrite {
      error_if_exists(path);
    }

    match tokio::fs::File::create(path).await {
      Ok(file) => Ok(Arc::new(Mutex::new(Box::new(file)))),

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

  eprintln!(
    "Error: Output file \"{}\" already exists.\n\nHint: Specify --overwrite to \
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
