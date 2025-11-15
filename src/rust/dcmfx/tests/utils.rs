use std::path::PathBuf;

use tempfile::TempDir;

fn temp_dir() -> PathBuf {
  if let Ok(t) = std::env::var("RUNNER_TEMP") {
    PathBuf::from(t)
  } else {
    std::env::temp_dir()
  }
}

#[allow(dead_code)]
pub fn create_temp_dir() -> TempDir {
  TempDir::new_in(temp_dir()).unwrap()
}
