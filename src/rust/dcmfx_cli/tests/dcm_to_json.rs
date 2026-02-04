mod utils;

use insta::assert_snapshot;
use utils::{create_temp_dir, dcmfx_cli, get_stdout, s3_get_object};

#[test]
fn with_output_filename() {
  let (input_path, output_path, _temp_dir) = prepare_temp_files(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
  );

  dcmfx_cli()
    .arg("dcm-to-json")
    .arg(&input_path)
    .arg("--output-filename")
    .arg(&output_path)
    .arg("--pretty")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", output_path.display()));

  assert_snapshot!(
    "with_output_filename",
    std::fs::read_to_string(&output_path).unwrap()
  );
}

#[test]
fn with_stdout_output() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--pretty")
    .assert()
    .success();

  assert_snapshot!("with_stdout_output", get_stdout(assert));
}

#[test]
fn with_stdout_output_and_strip_binary_values() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--pretty")
    .arg("--no-emit-binary-values")
    .assert()
    .success();

  assert_snapshot!(
    "with_stdout_output_and_strip_binary_values",
    get_stdout(assert)
  );
}

#[test]
fn with_stdout_output_and_strip_binary_values_raw() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--no-emit-binary-values")
    .assert()
    .success();

  assert_snapshot!(
    "with_stdout_output_and_strip_binary_values_raw",
    get_stdout(assert)
  );
}

#[tokio::test]
#[ignore]
async fn with_s3_input_and_output() {
  let input_file = "s3://dcmfx-test/pydicom/test_files/SC_rgb_small_odd.dcm";

  let output_key = format!("{}.json", rand::random::<u64>());
  let output_file = format!("s3://dcmfx-test/{output_key}");

  dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg(&output_file)
    .arg("--pretty")
    .assert()
    .success();

  let output_file = s3_get_object(&output_key).await;

  assert_snapshot!(
    "with_s3_input_and_output",
    std::fs::read_to_string(&output_file).unwrap()
  );
}

#[test]
fn with_selected_data_elements() {
  let dicom_files = [
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm",
  ];

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .args(dicom_files)
    .arg("--output-filename")
    .arg("-")
    .arg("--select")
    .arg("00080008")
    .arg("--select")
    .arg("00080016")
    .assert()
    .success();

  assert_snapshot!("with_selected_data_elements", get_stdout(assert));
}

#[test]
fn with_output_directory() {
  let dicom_files = [
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm",
  ];

  let output_directory = create_temp_dir();
  let output_files = [
    output_directory.path().join("SC_rgb_small_odd.dcm.json"),
    output_directory
      .path()
      .join("SC_rgb_small_odd_jpeg.dcm.json"),
  ];

  dcmfx_cli()
    .arg("dcm-to-json")
    .args(dicom_files)
    .arg("--pretty")
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
  let (input_path_0, output_path_0, _temp_dir_0) = prepare_temp_files(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm",
  );
  let (input_path_1, output_path_1, _temp_dir_1) = prepare_temp_files(
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd_jpeg.dcm",
  );

  dcmfx_cli()
    .arg("dcm-to-json")
    .args([input_path_0, input_path_1])
    .arg("--pretty")
    .arg("--concurrency")
    .arg("1")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n",
      output_path_0.display(),
      output_path_1.display()
    ));

  assert_snapshot!(
    "with_multiple_inputs_0",
    std::fs::read_to_string(&output_path_0).unwrap()
  );
  assert_snapshot!(
    "with_multiple_inputs_1",
    std::fs::read_to_string(&output_path_1).unwrap()
  );
}

fn prepare_temp_files(
  input_file: &str,
) -> (std::path::PathBuf, std::path::PathBuf, tempfile::TempDir) {
  let temp_dir = create_temp_dir();
  let input_path = temp_dir.path().join("input.dcm");
  let output_path = temp_dir.path().join("input.dcm.json");

  std::fs::copy(input_file, &input_path).unwrap();

  (input_path, output_path, temp_dir)
}
