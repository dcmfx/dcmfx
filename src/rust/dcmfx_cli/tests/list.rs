mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;
use utils::{get_stderr, get_stdout, to_native_path};

#[test]
fn with_multiple_directories() {
  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();

  let assert = cmd
    .arg("list")
    .arg("../../../test/assets/fo-dicom")
    .arg("../../../test/assets/pydicom")
    .assert()
    .success();

  let mut lines: Vec<_> =
    get_stdout(assert).split("\n").map(to_native_path).collect();
  lines.sort();

  #[cfg(windows)]
  assert_snapshot!("with_multiple_directories_windows", lines.join("\n"));

  #[cfg(not(windows))]
  assert_snapshot!("with_multiple_directories", lines.join("\n"));
}

#[test]
fn with_summary() {
  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();

  let assert = cmd
    .arg("list")
    .arg("../../../test/assets/fo-dicom")
    .arg("--extension")
    .arg("dcm")
    .arg("--summarize")
    .assert()
    .success();

  assert_eq!(
    get_stderr(assert).trim(),
    "Found 32 DICOM files, size: 11.2 MiB"
  );
}

#[test]
fn with_json_lines_format() {
  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();

  let assert = cmd
    .arg("list")
    .arg(to_native_path("../../../test/assets/fo-dicom"))
    .arg(to_native_path("../../../test/assets/pydicom"))
    .arg("--format")
    .arg("json-lines")
    .assert()
    .success();

  let stdout = get_stdout(assert);
  let mut lines: Vec<_> = stdout.split("\n").collect();
  lines.sort();

  #[cfg(windows)]
  assert_snapshot!("with_json_lines_format_windows", lines.join("\n"));

  #[cfg(not(windows))]
  assert_snapshot!("with_json_lines_format", lines.join("\n"));
}

#[test]
fn with_invalid_directory() {
  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();

  let assert = cmd.arg("list").arg("missing-directory").assert().failure();

  #[cfg(windows)]
  assert_snapshot!("with_invalid_directory_windows", get_stderr(assert));

  #[cfg(not(windows))]
  assert_snapshot!("with_invalid_directory", get_stderr(assert));
}
