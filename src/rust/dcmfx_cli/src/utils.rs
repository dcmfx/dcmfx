use std::{
  fs::File,
  io::Write,
  path::{Path, PathBuf},
};

use dcmfx::p10::P10Error;

/// Opens an output stream for the given path, first checking whether it exists
/// and prompting the user about overwriting it if necessary. This prompt isn't
/// presented to the user if `force_overwrite` is true.
///
/// The path "-" is interpreted as writing to stdout.
///
pub fn open_output_stream(
  path: &PathBuf,
  display_path: Option<&PathBuf>,
  force_overwrite: bool,
) -> Result<Box<dyn std::io::Write>, P10Error> {
  if *path == PathBuf::from("-") {
    Ok(Box::new(std::io::stdout()))
  } else {
    if let Some(display_path) = display_path {
      println!("Writing \"{}\" …", display_path.display());
    }

    if !force_overwrite {
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

/// Prompts the user about overwriting the given file if it exists.
///
pub fn prompt_to_overwrite_if_exists(path: &Path) {
  if !path.exists() {
    return;
  }

  print!(
    "File \"{}\" already exists. Overwrite? (y/N): ",
    path.display()
  );
  std::io::stdout().flush().unwrap();

  let mut input = String::new();
  std::io::stdin().read_line(&mut input).unwrap();
  let input = input.trim().to_lowercase();

  if input != "y" && input != "yes" {
    std::process::exit(1)
  }
}
