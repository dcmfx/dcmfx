mod utils;

use insta::assert_snapshot;

#[macro_use]
mod assert_image_snapshot;
use utils::{
  create_temp_dir, create_temp_file, dcmfx_cli, get_stderr, get_stdout,
  s3_copy_object,
};

#[test]
fn rewrite() {
  let temp_dir = create_temp_dir();
  let input_file = "../../../test/assets/fo-dicom/CT-MONO2-16-ankle.dcm";
  let output_file = temp_dir.path().join("output.dcm");

  let assert = dcmfx_cli().arg("print").arg(input_file).assert().success();

  assert_snapshot!("rewrite_before", get_stdout(assert));

  dcmfx_cli()
    .arg("rewrite")
    .arg(input_file)
    .arg("--output-filename")
    .arg(&output_file)
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!(
      "Rewriting \"{}\" => \"{}\" …\n",
      input_file,
      output_file.display()
    ));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(&output_file)
    .assert()
    .success();

  assert_snapshot!("rewrite_after", get_stdout(assert));
}

#[test]
fn rewrite_in_place() {
  let input_file = "../../../test/assets/fo-dicom/CR-MONO1-10-chest.dcm";
  let temp_file = create_temp_file();

  std::fs::copy(input_file, temp_file.path()).unwrap();

  let assert = dcmfx_cli()
    .arg("print")
    .arg(temp_file.path())
    .assert()
    .success();

  assert_snapshot!("rewrite_in_place_before", get_stdout(assert));

  dcmfx_cli()
    .arg("rewrite")
    .arg(temp_file.path())
    .arg("--in-place")
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!(
      "Rewriting \"{}\" in place …\n",
      temp_file.path().display()
    ));

  let assert = dcmfx_cli()
    .arg("print")
    .arg(temp_file.path())
    .assert()
    .success();

  assert_snapshot!("rewrite_in_place_after", get_stdout(assert));
}

#[tokio::test]
#[ignore]
async fn rewrite_in_place_on_s3() {
  let input_key = format!("{}.dcm", rand::random::<u64>());
  let input_file = format!("s3://dcmfx-test/{input_key}");

  s3_copy_object("fo-dicom/CR-MONO1-10-chest.dcm", &input_key).await;

  let assert = dcmfx_cli().arg("print").arg(&input_file).assert().success();

  assert_snapshot!("rewrite_in_place_before", get_stdout(assert));

  dcmfx_cli()
    .arg("rewrite")
    .arg(&input_file)
    .arg("--in-place")
    .arg("--implementation-version-name")
    .arg("DCMfx Test")
    .assert()
    .success()
    .stdout(format!("Rewriting \"{input_file}\" in place …\n"));

  let assert = dcmfx_cli().arg("print").arg(input_file).assert().success();

  assert_snapshot!("rewrite_in_place_after", get_stdout(assert));
}

#[test]
fn errors_on_missing_file() {
  let assert = dcmfx_cli()
    .arg("rewrite")
    .arg("--in-place")
    .arg("file-that-does-not-exist.dcm")
    .assert()
    .failure();

  assert_snapshot!("errors_on_missing_file", get_stderr(assert));
}
