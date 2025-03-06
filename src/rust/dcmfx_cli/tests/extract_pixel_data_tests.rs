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
fn dicom_rle_to_jpg() {
  let dicom_file = "../../../test/assets/pydicom/test_files/MR_small_RLE.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_rle_to_jpg.jpg");
}

#[test]
fn dicom_rle_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_32bit.dcm";
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

  assert_image_snapshot!(output_file, "dicom_rle_color_to_png.png");
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

#[test]
fn dicom_without_voi_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/rtdose_expb_1frame.dcm";
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

  assert_image_snapshot!(output_file, "dicom_without_voi_to_png.png");
}

#[test]
fn dicom_jpg_to_png() {
  let dicom_file = "../../../test/assets/fo-dicom/GH538-jpeg1.dcm";
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

  assert_image_snapshot!(output_file, "dicom_jpg_to_png.png");
}

#[test]
fn dicom_palette_color_to_png() {
  let dicom_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
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

  assert_image_snapshot!(output_file, "dicom_palette_color_to_png.png");
}

#[test]
fn dicom_jpeg_2000_single_channel_to_jpg() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("33904")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_rle_to_jpg.jpg");
}

#[test]
fn dicom_jpeg_2000_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/GDCMJ2K_TextGBR.dcm";
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

  assert_image_snapshot!(output_file, "dicom_jpeg_2000_color_to_png.png");
}

#[test]
fn dicom_jpeg_ls_single_channel_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/JPEGLSNearLossless_16.dcm";
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

  assert_image_snapshot!(
    output_file,
    "dicom_jpeg_ls_single_channel_to_png.png"
  );
}

#[test]
fn dicom_jpeg_ls_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_jls_lossy_sample.dcm";
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

  assert_image_snapshot!(output_file, "dicom_jpeg_ls_color_to_png.png");
}

#[test]
fn dicom_jpeg_lossless_12bit_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/IM-0001-0001-0001.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_jpeg_lossless_12bit_to_jpg.jpg");
}

#[test]
fn dicom_jpeg_lossless_color_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/GH538-jpeg14sv1.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("extract-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{output_file}\" …\n"));

  assert_image_snapshot!(output_file, "dicom_jpeg_lossless_color_to_jpg.jpg");
}

#[test]
fn dicom_jpeg_extended_12bit_single_channel_to_png() {
  let dicom_file = "../../../test/assets/pydicom/test_files/JPEG-lossy.dcm";
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

  assert_image_snapshot!(
    output_file,
    "dicom_jpeg_extended_12bit_single_channel_to_png.png"
  );
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
