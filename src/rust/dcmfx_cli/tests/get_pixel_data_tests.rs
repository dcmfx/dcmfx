use assert_cmd::Command;
use image::RgbImage;

// Macro to compare an image file to a snapshot
macro_rules! assert_image_snapshot {
  ($left:expr, $right:expr) => {
    assert_eq!(
      image_matches_snapshot($left, &format!("{}__{}", module_path!(), $right)),
      Ok(())
    )
  };
}

#[test]
fn single_bit_unaligned_to_raw() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let output_files = [
    (
      format!("{}.0000.bin", dicom_file),
      format!("{}.0000.png", dicom_file),
    ),
    (
      format!("{}.0001.bin", dicom_file),
      format!("{}.0001.png", dicom_file),
    ),
    (
      format!("{}.0002.bin", dicom_file),
      format!("{}.0002.png", dicom_file),
    ),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0].0),
      to_native_path(&output_files[1].0),
      to_native_path(&output_files[2].0)
    ));

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0].1),
      to_native_path(&output_files[1].1),
      to_native_path(&output_files[2].1)
    ));

  for i in 0..3 {
    let filename =
      format!("get_pixel_data_tests__single_bit_unaligned_to_raw.000{i}.bin");

    assert_eq!(
      std::fs::read(&output_files[i].0).unwrap(),
      std::fs::read(&format!("tests/snapshots/{filename}")).unwrap()
    );

    let filename = format!("single_bit_unaligned_to_raw.000{i}.png");
    assert_image_snapshot!(&output_files[i].1, &filename);
  }
}

#[test]
fn rgb_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rgb_to_png.png");
}

#[test]
fn ybr_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn rle_lossless_to_jpg() {
  let dicom_file = "../../../test/assets/pydicom/test_files/MR_small_RLE.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_to_jpg.jpg");
}

#[test]
fn rle_lossless_bitmap_to_png() {
  let dicom_file = "../../../test/assets/other/liver_1frame.rle_lossless.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_bitmap_to_png.png");
}

#[test]
fn rle_lossless_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_32bit.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_to_png.png");
}

#[test]
fn rle_lossless_color_palette_to_jpg() {
  let dicom_file =
    "../../../test/assets/other/TestPattern_Palette.rle_lossless.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_palette_to_jpg.jpg");
}

#[test]
fn to_jpg_with_custom_window() {
  let dicom_file = "../../../test/assets/fo-dicom/GH177_D_CLUNIE_CT1_IVRLE_BigEndian_ELE_undefinded_length.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("500")
    .arg("2000")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "to_jpg_with_custom_window.jpg");
}

#[test]
fn missing_voi_lut_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/rtdose_expb_1frame.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "missing_voi_lut_to_png.png");
}

#[test]
fn jpg_to_png() {
  let dicom_file = "../../../test/assets/fo-dicom/GH538-jpeg1.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpg_to_png.png");
}

#[test]
fn palette_color_to_png() {
  let dicom_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "palette_color_to_png.png");
}

#[test]
fn jpeg_2000_single_channel_to_jpg() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_to_jpg.jpg");
}

#[test]
fn jpeg_2000_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/GDCMJ2K_TextGBR.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_color_to_png.png");
}

#[test]
fn jpeg_2000_single_channel_with_mismatched_pixel_representation_to_jpg() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/J2K_pixelrep_mismatch.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_2000_single_channel_with_mismatched_pixel_representation_to_jpg.jpg"
  );
}

#[test]
fn jpeg_2000_single_channel_2bpp_to_png() {
  let dicom_file =
    "../../../test/assets/other/examples_jpeg2k.monochrome_2bpp.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_2000_single_channel_2bpp_to_png.png"
  );
}

#[test]
fn jpeg_ls_single_channel_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/JPEGLSNearLossless_16.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_single_channel_to_png.png");
}

#[test]
fn jpeg_ls_color_to_png() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_jls_lossy_sample.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_color_to_png.png");
}

#[test]
fn jpeg_lossless_12bit_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/IM-0001-0001-0001.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_12bit_to_jpg.jpg");
}

#[test]
fn jpeg_lossless_color_to_jpg() {
  let dicom_file = "../../../test/assets/fo-dicom/GH538-jpeg14sv1.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_color_to_jpg.jpg");
}

#[test]
fn jpeg_extended_12bit_single_channel_to_png() {
  let dicom_file = "../../../test/assets/pydicom/test_files/JPEG-lossy.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_extended_12bit_single_channel_to_png.png"
  );
}

#[test]
fn jpeg_xl_single_channel_to_png() {
  let dicom_file = "../../../test/assets/other/monochrome_jpeg_xl.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_single_channel_to_png.png");
}

#[test]
fn jpeg_xl_color_to_jpg() {
  let dicom_file = "../../../test/assets/other/ultrasound_jpeg_xl.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_color_to_jpg.jpg");
}

#[test]
fn deflated_image_frame_compression() {
  let dicom_file = "../../../test/assets/other/SC_ybr_full_422_uncompressed.deflated_image_frame_compression.dcm";
  let output_file = format!("{}.0000.png", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn render_overlays() {
  let dicom_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays.jpg");
}

#[test]
fn render_overlays_out_of_bounds() {
  let dicom_file = "../../../test/assets/fo-dicom/OutOfBoundsOverlay.dcm";
  let output_file = format!("{}.0000.jpg", dicom_file);

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays_out_of_bounds.jpg");
}

#[test]
fn render_overlays_multiframe_unaligned() {
  let dicom_file = "../../../test/assets/other/mr_brucker_with_unaligned_multiframe_overlay.dcm";
  let output_files = [
    format!("{}.0000.jpg", dicom_file),
    format!("{}.0001.jpg", dicom_file),
    format!("{}.0002.jpg", dicom_file),
    format!("{}.0003.jpg", dicom_file),
  ];

  let mut cmd = Command::cargo_bin("dcmfx_cli").unwrap();
  cmd
    .arg("get-pixel-data")
    .arg(dicom_file)
    .arg("-f")
    .arg("jpg")
    .arg("--overlays")
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\n\
       Writing \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0]),
      to_native_path(&output_files[1]),
      to_native_path(&output_files[2]),
      to_native_path(&output_files[3])
    ));

  assert_image_snapshot!(
    &output_files[1],
    "render_overlays_multiframe_unaligned.0001.jpg"
  );
  assert_image_snapshot!(
    &output_files[2],
    "render_overlays_multiframe_unaligned.0002.jpg"
  );
}

fn image_matches_snapshot<P: AsRef<std::path::Path>>(
  path1: P,
  snapshot: &str,
) -> Result<(), String> {
  let image_1: RgbImage = image::ImageReader::open(path1)
    .unwrap()
    .decode()
    .unwrap()
    .try_into()
    .unwrap();

  let image_snapshot_path = format!("tests/snapshots/{snapshot}");
  if !std::path::PathBuf::from(&image_snapshot_path).exists() {
    panic!("Snapshot file is missing: {image_snapshot_path}");
  }

  let image_2: RgbImage = image::ImageReader::open(image_snapshot_path)
    .unwrap()
    .decode()
    .unwrap()
    .try_into()
    .unwrap();

  if image_1.width() != image_2.width() || image_1.height() != image_2.height()
  {
    return Err(format!("Image dimensions don't match snapshot"));
  }

  // Check that the pixels are the same within a small epsilon
  for y in 0..image_1.height() {
    for x in 0..image_1.width() {
      let a = image_1.get_pixel(x, y);
      let b = image_2.get_pixel(x, y);

      if (a[0] as i16 - b[0] as i16).abs() > 2
        || (a[1] as i16 - b[1] as i16).abs() > 2
        || (a[2] as i16 - b[2] as i16).abs() > 2
      {
        return Err(format!(
          "Image differs at pixel {},{}: expected {:?} but got {:?}",
          x, y, b, a
        ));
      }
    }
  }

  Ok(())
}

fn to_native_path(path: &str) -> String {
  #[cfg(windows)]
  return path.replace("/", "\\");

  #[cfg(not(windows))]
  return path.to_string();
}
