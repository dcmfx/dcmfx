mod utils;

use assert_cmd::Command;
use insta::assert_snapshot;
use utils::{generate_temp_filename, get_stdout};

#[test]
fn with_output_filename() {
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
    "with_output_filename",
    std::fs::read_to_string(&temp_path).unwrap()
  );
}

#[test]
fn with_stdout_output() {
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

  assert_snapshot!("with_stdout_output", get_stdout(assert));
}

#[test]
fn with_output_directory() {
  let dicom_files = [
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm",
  ];

  let output_directory = generate_temp_filename();
  std::fs::create_dir(&output_directory).unwrap();
  let output_files = [
    output_directory.join("SC_rgb_small_odd.dcm.json"),
    output_directory.join("SC_rgb_small_odd_jpeg.dcm.json"),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("dcm-to-json")
    .args(dicom_files)
    .arg("--pretty")
    .arg("--output-directory")
    .arg(output_directory)
    .arg("--threads")
    .arg("1")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n",
      output_files[0].display(),
      output_files[1].display()
    ));

  assert_snapshot!(
    "with_output_directory_0",
    std::fs::read_to_string(&output_files[0]).unwrap()
  );
  assert_snapshot!(
    "with_output_directory_1",
    std::fs::read_to_string(&output_files[1]).unwrap()
  );
}

#[test]
fn with_multiple_inputs() {
  let dicom_files = [generate_temp_filename(), generate_temp_filename()];
  let output_files = [
    format!("{}{}", dicom_files[0].to_string_lossy(), ".json"),
    format!("{}{}", dicom_files[1].to_string_lossy(), ".json"),
  ];

  std::fs::copy(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
    &dicom_files[0],
  )
  .unwrap();
  std::fs::copy(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm",
    &dicom_files[1],
  )
  .unwrap();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("dcm-to-json")
    .args(dicom_files)
    .arg("--pretty")
    .arg("--threads")
    .arg("1")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n",
      output_files[0], output_files[1]
    ));

  assert_snapshot!(
    "with_multiple_inputs_0",
    std::fs::read_to_string(&output_files[0]).unwrap()
  );
  assert_snapshot!(
    "with_multiple_inputs_1",
    std::fs::read_to_string(&output_files[1]).unwrap()
  );
}
