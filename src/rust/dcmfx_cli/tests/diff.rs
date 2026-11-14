mod utils;

use insta::assert_snapshot;
use utils::{create_temp_dir, dcmfx_cli, get_stdout};

const JSON_A: &str = r#"{
  "00020010": { "vr": "UI", "Value": ["1.2.840.10008.1.2.1"] },
  "00100010": { "vr": "PN", "Value": [{ "Alphabetic": "Doe^Jane" }] },
  "00082112": {
    "vr": "SQ",
    "Value": [
      { "00080018": { "vr": "UI", "Value": ["1.2.3.4.0"] } },
      { "00080018": { "vr": "UI", "Value": ["1.2.3.4.1"] } }
    ]
  }
}"#;

const JSON_B: &str = r#"{
  "00020010": { "vr": "UI", "Value": ["1.2.840.10008.1.2.1"] },
  "00100010": { "vr": "PN", "Value": [{ "Alphabetic": "Doe^John" }] },
  "00082112": {
    "vr": "SQ",
    "Value": [
      { "00080018": { "vr": "UI", "Value": ["1.2.3.4.0"] } },
      { "00080018": { "vr": "UI", "Value": ["1.2.3.4.99"] } },
      { "00080018": { "vr": "UI", "Value": ["1.2.3.4.2"] } }
    ]
  }
}"#;

/// Converts the given DICOM JSON into a DICOM P10 file in the given temporary
/// directory, returning the path to the new file.
///
fn json_to_dcm(
  temp_dir: &std::path::Path,
  name: &str,
  json: &str,
) -> std::path::PathBuf {
  let json_path = temp_dir.join(format!("{name}.json"));
  std::fs::write(&json_path, json).unwrap();

  let dcm_path = temp_dir.join(format!("{name}.dcm"));

  dcmfx_cli()
    .arg("json-to-dcm")
    .arg(&json_path)
    .arg("--output-filename")
    .arg(&dcm_path)
    .assert()
    .success();

  dcm_path
}

#[test]
fn with_identical_files() {
  let temp_dir = create_temp_dir();
  let file_a = json_to_dcm(temp_dir.path(), "a", JSON_A);

  dcmfx_cli()
    .arg("diff")
    .arg(&file_a)
    .arg(&file_a)
    .assert()
    .success()
    .stdout("");
}

#[test]
fn with_differences() {
  let temp_dir = create_temp_dir();
  let file_a = json_to_dcm(temp_dir.path(), "a", JSON_A);
  let file_b = json_to_dcm(temp_dir.path(), "b", JSON_B);

  let assert = dcmfx_cli()
    .arg("diff")
    .arg(&file_a)
    .arg(&file_b)
    .assert()
    .failure();

  assert_snapshot!("with_differences", get_stdout(assert));
}

#[test]
fn with_nonexistent_input() {
  let temp_dir = create_temp_dir();
  let file_a = json_to_dcm(temp_dir.path(), "a", JSON_A);

  dcmfx_cli()
    .arg("diff")
    .arg(&file_a)
    .arg(temp_dir.path().join("does-not-exist.dcm"))
    .assert()
    .failure();
}
