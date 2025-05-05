mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;
use utils::{generate_temp_filename, get_stdout};

#[test]
fn print() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(dicom_file).assert().success();

  assert_snapshot!("print", get_stdout(assert));
}

#[test]
fn print_with_options() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("print")
    .arg("--max-width")
    .arg("200")
    .arg("--styled")
    .arg("true")
    .arg(dicom_file)
    .assert()
    .success();

  assert_snapshot!("print_with_options", get_stdout(assert));
}

#[test]
fn print_multiple() {
  let dicom_file_0 = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";
  let dicom_file_1 = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("print")
    .arg(dicom_file_0)
    .arg(dicom_file_1)
    .assert()
    .success();

  assert_snapshot!("print_multiple", get_stdout(assert));
}

#[test]
fn json_to_dcm() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json";
  let temp_path = generate_temp_filename();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("json-to-dcm")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", temp_path.display()));

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(temp_path).assert();

  assert_snapshot!("json_to_dcm", get_stdout(assert));
}

#[test]
fn dcm_to_json_on_stdout() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("dcm-to-json")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--pretty")
    .assert()
    .success();

  assert_snapshot!("dcm_to_json_on_stdout", get_stdout(assert));
}

#[test]
fn dcm_to_json() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";
  let temp_path = generate_temp_filename();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("dcm-to-json")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .arg("--pretty")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", temp_path.display()));

  assert_snapshot!("dcm_to_json", std::fs::read_to_string(&temp_path).unwrap());
}
