use std::{
  fs::File,
  io::Write,
  path::{Path, PathBuf},
  sync::atomic::{AtomicBool, Ordering},
};

use dcmfx::p10::P10Error;

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
      prompt_to_overwrite_if_exists(path);
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

/// Stores whether the user has requested to overwrite all files instead of
/// prompting for each one.
static OVERWRITE_ALL_FILES: AtomicBool = AtomicBool::new(false);

/// Prompts the user about overwriting the given file if it exists.
///
pub fn prompt_to_overwrite_if_exists(path: &Path) {
  if !path.exists() {
    return;
  }

  if OVERWRITE_ALL_FILES.load(Ordering::Relaxed) {
    return;
  }

  print!(
    "File \"{}\" already exists. Overwrite? ([y]es, [n]o, [a]ll): ",
    path.display()
  );
  std::io::stdout().flush().unwrap();

  let mut input = String::new();
  std::io::stdin().read_line(&mut input).unwrap();
  let input = input.trim().to_lowercase();

  if input != "y" && input != "yes" && input != "a" && input != "all" {
    std::process::exit(1)
  }

  if input == "a" || input == "all" {
    OVERWRITE_ALL_FILES.store(true, Ordering::Relaxed);
  }
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
