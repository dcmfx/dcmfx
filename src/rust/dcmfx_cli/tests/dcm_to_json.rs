mod utils;

use bytes::Bytes;
use insta::assert_snapshot;
use utils::{create_temp_dir, dcmfx_cli, get_stdout, s3_get_object};

use dcmfx::{core::*, p10::*};

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
  let input_file = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--exclude-binary-values")
    .assert()
    .success();

  assert_snapshot!(
    "with_stdout_output_and_strip_binary_values",
    get_stdout(assert)
  );
}

#[test]
fn with_stdout_output_and_strip_binary_values_pretty_print() {
  let input_file = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--exclude-binary-values")
    .arg("--pretty")
    .assert()
    .success();

  assert_snapshot!(
    "with_stdout_output_and_strip_binary_values_pretty_print",
    get_stdout(assert)
  );
}

#[test]
fn with_stdout_output_and_include_specific_binary_values() {
  let input_file = "../../../test/assets/fo-dicom/CT1_J2KI.dcm";

  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(input_file)
    .arg("--output-filename")
    .arg("-")
    .arg("--pretty")
    .arg("--select-binary-value")
    .arg("00431029")
    .arg("--select-binary-value")
    .arg("0043102A")
    .assert()
    .success();

  assert_snapshot!(
    "with_stdout_output_and_include_specific_binary_values",
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

#[test]
fn with_ignore_invalid_data() {
  // Create a DICOM P10 file with an invalid '(0020,0011) Series Number' data
  // element
  let data_set: DataSet = [(
    dictionary::SERIES_NUMBER.tag,
    DataElementValue::new_binary_unchecked(
      ValueRepresentation::IntegerString,
      Bytes::from(b"invalid ".to_vec()),
    ),
  )]
  .into_iter()
  .collect();

  let temp_dir = create_temp_dir();
  let input_path = temp_dir.path().join("input.dcm");
  data_set.write_p10_file(&input_path, None).unwrap();

  // Without the flag the invalid data causes the command to fail
  dcmfx_cli()
    .arg("dcm-to-json")
    .arg(&input_path)
    .arg("--output-filename")
    .arg("-")
    .assert()
    .failure();

  // With the flag the invalid data element is emitted without a value and the
  // command succeeds
  let assert = dcmfx_cli()
    .arg("dcm-to-json")
    .arg(&input_path)
    .arg("--output-filename")
    .arg("-")
    .arg("--ignore-invalid-data")
    .arg("00200011")
    .assert()
    .success();

  assert_eq!(get_stdout(assert).trim_end(), r#"{"00200011":{"vr":"IS"}}"#);
}

#[test]
fn with_ignore_invalid_vr_and_private() {
  // Create a DICOM P10 file with private FL and UL data elements and a public
  // FD data element, all of which have an invalid length. Explicit VR is used
  // so that the private data elements' VRs are preserved.
  let invalid_value = |vr| {
    DataElementValue::new_binary_unchecked(vr, Bytes::from_static(&[0, 0, 0]))
  };

  let data_set: DataSet = [
    (
      dictionary::TRANSFER_SYNTAX_UID.tag,
      DataElementValue::new_unique_identifier(&[
        transfer_syntax::EXPLICIT_VR_LITTLE_ENDIAN.uid,
      ])
      .unwrap(),
    ),
    (
      DataElementTag::new(0x0009, 0x1001),
      invalid_value(ValueRepresentation::FloatingPointSingle),
    ),
    (
      DataElementTag::new(0x0009, 0x1002),
      invalid_value(ValueRepresentation::UnsignedLong),
    ),
    (
      dictionary::DIFFUSION_B_VALUE.tag,
      invalid_value(ValueRepresentation::FloatingPointDouble),
    ),
  ]
  .into_iter()
  .collect();

  let temp_dir = create_temp_dir();
  let input_path = temp_dir.path().join("input.dcm");
  data_set.write_p10_file(&input_path, None).unwrap();

  let run = |args: &[&str]| {
    dcmfx_cli()
      .arg("dcm-to-json")
      .arg(&input_path)
      .arg("--output-filename")
      .arg("-")
      .args(args)
      .assert()
  };

  // Ignoring all private data elements doesn't cover the public FD
  run(&["--ignore-invalid-private"]).failure();

  // Ignoring FL and FD everywhere doesn't cover the private UL
  run(&["--ignore-invalid-vr", "FL", "--ignore-invalid-vr", "FD"]).failure();

  // Ignoring private FL, UL, and FD doesn't cover the public FD
  run(&[
    "--ignore-invalid-private-vr",
    "FL",
    "--ignore-invalid-private-vr",
    "UL",
    "--ignore-invalid-private-vr",
    "FD",
  ])
  .failure();

  // Ignoring all private data elements, and FL and FD everywhere, succeeds
  let assert = run(&[
    "--ignore-invalid-private",
    "--ignore-invalid-vr",
    "fl",
    "--ignore-invalid-vr",
    "FD",
  ])
  .success();

  assert_eq!(
    get_stdout(assert).trim_end(),
    concat!(
      r#"{"00020010":{"vr":"UI","Value":["1.2.840.10008.1.2.1"]},"#,
      r#""00091001":{"vr":"FL"},"00091002":{"vr":"UL"},"#,
      r#""00189087":{"vr":"FD"}}"#
    )
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
