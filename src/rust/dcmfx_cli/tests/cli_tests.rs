mod utils;

use assert_cmd::{Command, assert::Assert};
use insta::assert_snapshot;
use rand::Rng;
use utils::to_native_path;

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
fn json_to_dcm_to_file() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json";
  let temp_path = generate_temp_filename();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("json-to-dcm")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", temp_path.display()));

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(temp_path).assert();

  assert_snapshot!("json_to_dcm_to_file", get_stdout(assert));
}

#[test]
fn dcm_to_json_to_stdout() {
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

  assert_snapshot!("dcm_to_json_to_stdout", get_stdout(assert));
}

#[test]
fn dcm_to_json_to_file() {
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

  assert_snapshot!(
    "dcm_to_json_to_file",
    std::fs::read_to_string(&temp_path).unwrap()
  );
}

#[test]
fn modify() {
  let dicom_file = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";
  let temp_path = generate_temp_filename();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(dicom_file)
    .assert()
    .success();

  assert_snapshot!("modify_before", get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg("explicit-vr-big-endian")
    .arg("--anonymize")
    .arg(dicom_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .assert()
    .success()
    .stdout(format!(
      "Modifying \"{}\" => \"{}\" …\n",
      to_native_path(&dicom_file),
      temp_path.display()
    ));

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_after", get_stdout(assert));
}

#[test]
fn modify_in_place() {
  let dicom_file = "../../../test/assets/fo-dicom/CR-MONO1-10-chest.dcm";
  let temp_path = generate_temp_filename();

  std::fs::copy(dicom_file, &temp_path).unwrap();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_in_place_before", get_stdout(assert));

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg("deflated-explicit-vr-little-endian")
    .arg("--in-place")
    .arg(&temp_path)
    .assert()
    .success()
    .stdout(format!(
      "Modifying \"{}\" in place …\n",
      temp_path.display()
    ));

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(&temp_path)
    .assert()
    .success();

  assert_snapshot!("modify_in_place_after", get_stdout(assert));
}

fn generate_temp_filename() -> std::path::PathBuf {
  let temp_dir = std::env::temp_dir();

  let mut rng = rand::rng();
  let random_suffix: String = (0..16)
    .map(|_| char::from(rng.sample(rand::distr::Alphanumeric)))
    .collect();

  let file_name = format!("dcmfx_{}", random_suffix);
  temp_dir.join(file_name)
}

fn get_stdout(assert: Assert) -> String {
  String::from_utf8(assert.get_output().stdout.clone()).unwrap()
}
