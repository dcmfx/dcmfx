mod utils;

use insta::assert_snapshot;

use utils::{create_temp_dir, dcmfx_cli, get_stdout, s3_get_object};

use crate::utils::create_temp_file;

#[test]
fn with_output_filename() {
  let temp_dir = create_temp_dir();
  let input_path = temp_dir.path().join("input.json");
  let output_path = temp_dir.path().join("output.dcm");

  std::fs::copy(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json",
    &input_path,
  )
  .unwrap();

  dcmfx_cli()
    .arg("json-to-dcm")
    .arg(&input_path)
    .arg("--output-filename")
    .arg(&output_path)
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", output_path.display()));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_path)
    .assert()
    .success();

  assert_snapshot!("with_output_filename", get_stdout(assert));
}

#[test]
fn with_stdout_output() {
  let dicom_json_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json";

  let assert = dcmfx_cli()
    .arg("json-to-dcm")
    .arg(dicom_json_file)
    .arg("--output-filename")
    .arg("-")
    .assert()
    .success();

  let p10_data = assert.get_output().stdout.clone();

  let temp_file = create_temp_file();
  std::fs::write(temp_file.path(), p10_data).unwrap();

  let assert = dcmfx_cli()
    .arg("print")
    .arg(temp_file.path())
    .assert()
    .success();

  assert_snapshot!("with_stdout_output", get_stdout(assert));
}

#[test]
fn with_output_directory() {
  let output_directory = create_temp_dir();

  let input_files = [
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json",
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm.json",
  ];

  let output_files = [
    output_directory
      .path()
      .join("SC_rgb_small_odd.dcm.json.dcm"),
    output_directory
      .path()
      .join("SC_rgb_small_odd_jpeg.dcm.json.dcm"),
  ];

  dcmfx_cli()
    .arg("json-to-dcm")
    .args(input_files)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("--concurrency")
    .arg("1")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n",
      output_files[0].display(),
      output_files[1].display()
    ));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_files[0])
    .assert()
    .success();

  assert_snapshot!("with_output_directory_0", get_stdout(assert));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_files[1])
    .assert()
    .success();

  assert_snapshot!("with_output_directory_1", get_stdout(assert));
}

#[test]
fn with_multiple_inputs() {
  let temp_dir = create_temp_dir();
  let input_files = [
    temp_dir.path().join("1.json"),
    temp_dir.path().join("2.json"),
  ];
  let output_files = [
    format!("{}{}", input_files[0].display(), ".dcm"),
    format!("{}{}", input_files[1].display(), ".dcm"),
  ];

  std::fs::copy(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm.json",
    &input_files[0],
  )
  .unwrap();
  std::fs::copy(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm.json",
    &input_files[1],
  )
  .unwrap();

  dcmfx_cli()
    .arg("json-to-dcm")
    .args(input_files)
    .arg("--concurrency")
    .arg("1")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n",
      output_files[0], output_files[1]
    ));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_files[0])
    .assert()
    .success();

  assert_snapshot!("with_multiple_inputs_0", get_stdout(assert));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_files[1])
    .assert()
    .success();

  assert_snapshot!("with_multiple_inputs_1", get_stdout(assert));
}

#[tokio::test]
#[ignore]
async fn with_s3_input_and_output() {
  let input_file =
    "s3://dcmfx-test/pydicom/test_files/SC_rgb_small_odd.dcm.json";

  let output_key = format!("{}.dcm", rand::random::<u64>());
  let output_file = format!("s3://dcmfx-test/{output_key}");

  dcmfx_cli()
    .arg("json-to-dcm")
    .arg(input_file)
    .arg("--output-filename")
    .arg(&output_file)
    .assert()
    .success();

  let output_file = s3_get_object(&output_key).await;

  let assert = dcmfx_cli()
    .arg("print")
    .arg(output_file.path())
    .assert()
    .success();

  assert_snapshot!("with_s3_input_and_output", get_stdout(assert));
}
