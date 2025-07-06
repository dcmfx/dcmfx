mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;
use utils::get_stdout;

use crate::utils::generate_temp_filename;

#[test]
fn with_single_input() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(dicom_file).assert().success();

  assert_snapshot!("with_single_input", get_stdout(assert));
}

#[test]
fn with_style_options() {
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

  assert_snapshot!("with_style_options", get_stdout(assert));
}

#[test]
fn with_multiple_inputs() {
  let dicom_file_0 = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";
  let dicom_file_1 = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("print")
    .arg(dicom_file_0)
    .arg(dicom_file_1)
    .assert()
    .success();

  assert_snapshot!("with_multiple_inputs", get_stdout(assert));
}

#[test]
fn with_file_list() {
  let file_list = generate_temp_filename();
  std::fs::write(
    &file_list,
    "
../../../test/assets/fo-dicom/CT1_J2KI.dcm
  

 ../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm  
 
",
  )
  .unwrap();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("print")
    .arg("--file-list")
    .arg(file_list)
    .assert()
    .success();

  assert_snapshot!("with_file_list", get_stdout(assert));
}
