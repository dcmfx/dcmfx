use assert_cmd::Command;
use image::RgbImage;

// Macro to compare an image file to a snapshot
macro_rules! assert_image_snapshot {
  ($left:expr, $right:expr) => {
    assert!(image_matches_snapshot(
      $left,
      &format!("{}__{}", module_path!(), $right)
    ))
  };
}

#[test]
fn dicom_rgb_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_rgb_to_png.png");
}

#[test]
fn dicom_ybr_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_ybr_to_png.png");
}

#[test]
fn dicom_to_jpg_with_custom_window() {
  let dicom_file = "../../../test/assets/fo-dicom/GH177_D_CLUNIE_CT1_IVRLE_BigEndian_ELE_undefinded_length.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("500")
    .arg("2000")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_to_jpg_with_custom_window.jpg");
}

fn image_matches_snapshot<P: AsRef<std::path::Path>>(
  path1: P,
  snapshot: &str,
) -> bool {
  let image_1: RgbImage = image::ImageReader::open(path1)
    .unwrap()
    .decode()
    .unwrap()
    .try_into()
    .unwrap();

  let image_2: RgbImage =
    image::ImageReader::open(format!("tests/snapshots/{snapshot}"))
      .unwrap()
      .decode()
      .unwrap()
      .try_into()
      .unwrap();

  // Check that the pixels are the same within a small epsilon
  for (a, b) in image_1.pixels().zip(image_2.pixels()) {
    if (a[2] as i16 - b[2] as i16).abs() > 1 {
      return false;
    }
  }

  true
}
