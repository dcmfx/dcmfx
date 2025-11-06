mod utils;

use assert_cmd::Command;
use dcmfx::{
  core::DataSet,
  json::{DataSetJsonExtensions, DicomJsonConfig},
  p10::DataSetP10Extensions,
};
use insta::assert_snapshot;
use utils::{generate_temp_filename, get_stdout};

const JSON_CONFIG: DicomJsonConfig = DicomJsonConfig {
  store_encapsulated_pixel_data: true,
  pretty_print: true,
};

#[test]
fn with_output_filename() {
  let dicom_json_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json";
  let temp_path = generate_temp_filename();

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("json-to-dcm")
    .arg(dicom_json_file)
    .arg("--output-filename")
    .arg(&temp_path)
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", temp_path.display()));

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd.arg("print").arg(temp_path).assert();

  assert_snapshot!("with_output_filename", get_stdout(assert));
}

#[test]
fn with_stdout_output() {
  let dicom_json_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json";

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  let assert = cmd
    .arg("json-to-dcm")
    .arg(dicom_json_file)
    .arg("--output-filename")
    .arg("-")
    .assert()
    .success();

  let p10_data = assert.get_output().stdout.clone();

  // Convert stdout data back to JSON so it can be asserted
  let json = DataSet::read_p10_bytes(p10_data.into())
    .unwrap()
    .to_json(JSON_CONFIG)
    .unwrap();

  assert_snapshot!("with_stdout_output", json);
}

#[test]
fn with_output_directory() {
  let dicom_json_files = [
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json",
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm.json",
  ];

  let output_directory = generate_temp_filename();
  std::fs::create_dir(&output_directory).unwrap();
  let output_files = [
    output_directory.join("SC_rgb_small_odd.dcm.json.dcm"),
    output_directory.join("SC_rgb_small_odd_jpeg.dcm.json.dcm"),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("json-to-dcm")
    .args(dicom_json_files)
    .arg("--output-directory")
    .arg(output_directory)
    .arg("--concurrency")
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
    DataSet::read_p10_file(&output_files[0])
      .unwrap()
      .to_json(JSON_CONFIG)
      .unwrap()
  );
  assert_snapshot!(
    "with_output_directory_1",
    DataSet::read_p10_file(&output_files[1])
      .unwrap()
      .to_json(JSON_CONFIG)
      .unwrap()
  );
}

#[test]
fn with_multiple_inputs() {
  let dicom_files = [generate_temp_filename(), generate_temp_filename()];
  let output_files = [
    format!("{}{}", dicom_files[0].display(), ".json"),
    format!("{}{}", dicom_files[1].display(), ".json"),
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
    .arg("--concurrency")
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
