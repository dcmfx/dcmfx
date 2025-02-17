use assert_cmd::Command;
use insta::assert_snapshot;

#[test]
fn print() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(dicom_file).assert();

  let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert_snapshot!("print", output);
}

#[test]
fn to_json() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("to-json")
    .arg(dicom_file)
    .arg("-")
    .arg("--pretty")
    .assert();

  let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert_snapshot!("to_json", output);
}

#[test]
fn modify() {
  let dicom_file = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";
  let output_file = "out.dcm";

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(dicom_file)
    .assert()
    .success();

  let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert_snapshot!("modify_before", output);

  Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("modify")
    .arg("--transfer-syntax")
    .arg("explicit-vr-big-endian")
    .arg("--anonymize")
    .arg(dicom_file)
    .arg(output_file)
    .assert()
    .success();

  let assert = Command::cargo_bin("dcmfx_cli")
    .unwrap()
    .arg("print")
    .arg(output_file)
    .assert()
    .success();

  let output = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
  assert_snapshot!("modify_after", output);

  std::fs::remove_file(output_file).unwrap()
}
