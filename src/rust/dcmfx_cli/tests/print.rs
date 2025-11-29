mod utils;

use insta::assert_snapshot;
use utils::{create_temp_file, dcmfx_cli, get_stdout};

#[test]
fn with_single_input() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli().arg("print").arg(input_file).assert().success();

  assert_snapshot!("with_single_input", get_stdout(assert));
}

#[test]
#[ignore]
fn with_single_s3_input() {
  let input_file = "s3://dcmfx-test/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli().arg("print").arg(input_file).assert().success();

  assert_snapshot!("with_single_s3_input", get_stdout(assert));
}

#[test]
fn with_style_options() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli()
    .arg("print")
    .arg("--max-width")
    .arg("200")
    .arg("--styled")
    .arg("true")
    .arg(input_file)
    .assert()
    .success();

  assert_snapshot!("with_style_options", get_stdout(assert));
}

#[test]
fn with_multiple_inputs() {
  let dicom_file_0 = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";
  let dicom_file_1 = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";

  let assert = dcmfx_cli()
    .arg("print")
    .arg(dicom_file_0)
    .arg(dicom_file_1)
    .assert()
    .success();

  assert_snapshot!("with_multiple_inputs", get_stdout(assert));
}

#[test]
fn with_file_list() {
  let file_list = create_temp_file();
  std::fs::write(
    file_list.path(),
    "
../../../test/assets/fo-dicom/CT1_J2KI.dcm
  

 ../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm  
 
",
  )
  .unwrap();

  let assert = dcmfx_cli()
    .arg("print")
    .arg("--file-list")
    .arg(file_list.path())
    .assert()
    .success();

  assert_snapshot!("with_file_list", get_stdout(assert));
}

#[test]
fn with_file_list_containing_nonexistent_file() {
  let file_list = create_temp_file();
  std::fs::write(file_list.path(), "file-that-does-not-exist.dcm").unwrap();

  dcmfx_cli()
    .arg("print")
    .arg("--file-list")
    .arg(file_list.path())
    .assert()
    .failure();
}

#[test]
fn with_default_transfer_syntax() {
  let dicom_p10 = std::fs::read(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
  )
  .unwrap()[0x156..]
    .to_vec();

  dcmfx_cli()
    .arg("print")
    .arg("-")
    .write_stdin(dicom_p10.clone())
    .assert()
    .failure();

  let assert = dcmfx_cli()
    .arg("print")
    .arg("--default-transfer-syntax")
    .arg("1.2.840.10008.1.2.1")
    .arg("-")
    .write_stdin(dicom_p10)
    .assert()
    .success();

  assert_snapshot!("with_default_transfer_syntax", get_stdout(assert));
}

#[test]
#[ignore]
fn with_s3_glob_input() {
  let local_glob_stdout =
    print_command_stdout_sorted("../../../test/assets/pydicom/palettes/*.dcm");
  assert_snapshot!("with_s3_glob_input", local_glob_stdout);

  assert_eq!(
    local_glob_stdout,
    print_command_stdout_sorted("s3://dcmfx-test/pydicom/palettes/*.dcm")
  );
}

#[test]
#[ignore]
fn with_s3_glob_input_with_partial_prefix() {
  let local_glob_stdout = print_command_stdout_sorted(
    "../../../test/assets/pydicom/palettes/pet*.dcm",
  );
  assert_snapshot!("with_s3_glob_input_with_partial_prefix", local_glob_stdout);

  assert_eq!(
    local_glob_stdout,
    print_command_stdout_sorted("s3://dcmfx-test/pydicom/palettes/pet*.dcm")
  );
}

fn print_command_stdout_sorted(input_file: &str) -> String {
  use itertools::Itertools;

  let assert = dcmfx_cli().arg("print").arg(input_file).assert().success();

  get_stdout(assert)
    .lines()
    .sorted()
    .collect::<Vec<_>>()
    .join("\n")
}
