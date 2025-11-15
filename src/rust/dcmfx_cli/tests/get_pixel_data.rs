mod utils;

use assert_cmd::Command;

#[macro_use]
mod assert_image_snapshot;
use tempfile::TempDir;
use utils::{create_temp_dir, dcmfx_cli, s3_get_object, to_native_path};

#[test]
fn single_bit_unaligned() {
  let input_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let (output_file, output_directory) = prepare_outputs(input_file, "");

  let output_files = [
    (
      format!("{output_file}.0000.bin"),
      format!("{output_file}.0000.png"),
    ),
    (
      format!("{output_file}.0001.bin"),
      format!("{output_file}.0001.png"),
    ),
    (
      format!("{output_file}.0002.bin"),
      format!("{output_file}.0002.png"),
    ),
  ];

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .assert()
    .success()
    .stdout(format!(
      "Writing \"{}\" …\nWriting \"{}\" …\nWriting \"{}\" …\n",
      to_native_path(&output_files[0].0),
      to_native_path(&output_files[1].0),
      to_native_path(&output_files[2].0)
    ));

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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

  for (i, output_file) in output_files.iter().enumerate() {
    let filename = format!("get_pixel_data__single_bit_unaligned.000{i}.bin");

    assert_eq!(
      std::fs::read(&output_file.0).unwrap(),
      std::fs::read(format!("tests/snapshots/{filename}")).unwrap()
    );

    let filename = format!("single_bit_unaligned.000{i}.png");
    assert_image_snapshot!(&output_file.1, &filename);
  }
}

#[test]
fn single_bit_unaligned_cropped_to_png() {
  let input_file =
    "../../../test/assets/other/liver_nonbyte_aligned_cropped.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success();

  assert_image_snapshot!(
    output_file,
    "single_bit_unaligned_cropped_to_png.png"
  );
}

#[test]
fn rgb_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_small_odd.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rgb_to_png.png");
}

#[test]
fn ybr_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_ybr_full_422_uncompressed.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn modality_lut_sequence() {
  let input_file = "../../../test/assets/fo-dicom/CR-ModalitySequenceLUT.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "modality_lut_sequence.png");
}

#[test]
fn rle_lossless_to_jpg() {
  let input_file = "../../../test/assets/pydicom/test_files/MR_small_RLE.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
  let input_file = "../../../test/assets/other/liver_1frame.rle_lossless.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_bitmap_to_png.png");
}

#[test]
fn rle_lossless_color_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_rle_32bit.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_to_png.png");
}

#[test]
fn rle_lossless_color_palette_to_jpg() {
  let input_file =
    "../../../test/assets/other/TestPattern_Palette.rle_lossless.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "rle_lossless_color_palette_to_jpg.jpg");
}

#[test]
fn to_jpg_with_custom_window() {
  let input_file = "../../../test/assets/fo-dicom/GH177_D_CLUNIE_CT1_IVRLE_BigEndian_ELE_undefinded_length.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
  let input_file =
    "../../../test/assets/pydicom/test_files/rtdose_expb_1frame.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "missing_voi_lut_to_png.png");
}

#[test]
fn palette_color_to_png() {
  let input_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "palette_color_to_png.png");
}

#[test]
fn resize_using_lanczos3() {
  let input_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .arg("--resize")
    .arg("100")
    .arg("0")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "resize_using_lanczos3_filter.png");
}

#[test]
fn resize_using_bilinear_filter() {
  let input_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .arg("--resize")
    .arg("100")
    .arg("0")
    .arg("--resize-filter")
    .arg("bilinear")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "resize_using_bilinear_filter.png");
}

#[test]
fn crop() {
  let input_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .arg("--crop")
    .arg("100,50,50,80")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "crop.png");
}

#[test]
fn crop_and_transform_and_resize() {
  let input_file = "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .arg("--crop")
    .arg("100,50")
    .arg("--transform")
    .arg("flip-vertical")
    .arg("--resize")
    .arg("500")
    .arg("100")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "crop_and_transform_and_resize.png");
}

#[test]
fn jpeg_2000_monochrome_to_jpg() {
  let input_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
fn jpeg_2000_monochrome_to_png_16bit() {
  let input_file =
    "../../../test/assets/pydicom/test_files/MR_small_jp2klossless.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png16")
    .arg("--voi-window")
    .arg("1136")
    .arg("2018")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_monochrome_to_png_16bit.png");
}

#[test]
fn jpeg_2000_color_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/GDCMJ2K_TextGBR.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(&output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_color_to_png.png");
}

#[test]
fn jpeg_2000_ybr_color_space_to_jpg() {
  let input_file = "../../../test/assets/other/jpeg_2000_ybr_color_space.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_ybr_color_space_to_jpg.jpg");
}

#[test]
fn jpeg_2000_monochrome_with_mismatched_pixel_representation_to_jpg() {
  let input_file =
    "../../../test/assets/pydicom/test_files/J2K_pixelrep_mismatch.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_2000_monochrome_with_mismatched_pixel_representation_to_jpg.jpg"
  );
}

#[test]
fn jpeg_2000_monochrome_2bpp_to_png() {
  let input_file =
    "../../../test/assets/other/examples_jpeg_2000.monochrome_2bpp.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_2000_monochrome_2bpp_to_png.png");
}

#[test]
fn jpeg_ls_monochrome_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/JPEGLSNearLossless_16.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_monochrome_to_png.png");
}

#[test]
fn jpeg_ls_color_to_png() {
  let input_file =
    "../../../test/assets/pydicom/test_files/SC_rgb_jls_lossy_sample.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_color_to_png.png");
}

#[test]
fn jpeg_ls_ybr_color_space_to_jpg() {
  let input_file = "../../../test/assets/other/jpeg_ls_ybr_color_space.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_ybr_color_space_to_jpg.jpg");
}

#[test]
fn jpeg_ls_palette_color_to_png() {
  let input_file =
    "../../../test/assets/other/TestPattern_Palette_16.jpeg_ls_lossless.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_ls_palette_color_to_png.png");
}

#[test]
fn jpeg_lossless_12bit_to_jpg() {
  let input_file = "../../../test/assets/fo-dicom/IM-0001-0001-0001.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_12bit_to_jpg.jpg");
}

#[test]
fn jpeg_lossless_color_to_jpg() {
  let input_file = "../../../test/assets/fo-dicom/GH538-jpeg14sv1.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_lossless_color_to_jpg.jpg");
}

#[test]
fn jpeg_lossless_12bit_to_jpg_with_inverse_presentation_lut_shape() {
  let input_file = "../../../test/assets/other/jpeg_lossless_with_inverse_presentation_lut_shape.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_lossless_12bit_to_jpg_with_inverse_presentation_lut_shape.jpg"
  );
}

#[test]
fn jpeg_lossless_12bit_to_jpg_with_presentation_lut() {
  let input_file =
    "../../../test/assets/other/jpeg_lossless_with_presentation_lut.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("jpg")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_lossless_12bit_to_jpg_with_presentation_lut.jpg"
  );
}

#[test]
fn jpeg_extended_12bit_monochrome_to_png() {
  let input_file = "../../../test/assets/pydicom/test_files/JPEG-lossy.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "jpeg_extended_12bit_monochrome_to_png.png"
  );
}

#[test]
fn jpeg_xl_monochrome_to_png() {
  let input_file = "../../../test/assets/other/monochrome_jpeg_xl.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_monochrome_to_png.png");
}

#[test]
fn jpeg_xl_monochrome_12bit_to_png() {
  let input_file = "../../../test/assets/other/monochrome_jpeg_xl_12bit.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .arg("--jpeg-xl-decoder")
    .arg("jxl-oxide")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_monochrome_12bit_to_png.png");
}

#[test]
fn jpeg_xl_color_to_jpg() {
  let input_file = "../../../test/assets/other/ultrasound_jpeg_xl.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "jpeg_xl_color_to_png.png");
}

#[test]
fn deflated_image_frame_compression() {
  let input_file = "../../../test/assets/other/SC_ybr_full_422_uncompressed.deflated_image_frame_compression.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "ybr_to_png.png");
}

#[test]
fn render_overlays() {
  let input_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
  let input_file = "../../../test/assets/fo-dicom/OutOfBoundsOverlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
  let input_file = "../../../test/assets/other/mr_brucker_with_unaligned_multiframe_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.jpg");

  let output_files = [
    output_file.clone(),
    output_file.replace(".0000.jpg", ".0001.jpg"),
    output_file.replace(".0000.jpg", ".0002.jpg"),
    output_file.replace(".0000.jpg", ".0003.jpg"),
  ];

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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

#[test]
fn single_bit_unaligned_to_mp4_h264() {
  let input_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let (output_file, output_directory) = prepare_outputs(input_file, ".mp4");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("mp4")
    .arg("--mp4-preset")
    .arg("fast")
    .arg("--mp4-pixel-format")
    .arg("yuv420p10")
    .assert()
    .success()
    .stdout(format!("Writing \"{0}\" …\n", to_native_path(&output_file),));

  assert_eq!(
    get_video_stream_details(&output_file),
    Ok(VideoStreamDetails {
      codec_name: "h264".to_string(),
      profile: "High 10".to_string(),
      width: 510,
      height: 510,
      pix_fmt: "yuv420p10le".to_string(),
      r_frame_rate: "1/1".to_string(),
    })
  );

  assert_eq!(get_video_frame_count(&output_file), Ok(3));
}

#[test]
fn single_bit_unaligned_to_mp4_h265() {
  let input_file =
    "../../../test/assets/pydicom/test_files/liver_nonbyte_aligned.dcm";
  let (output_file, output_directory) = prepare_outputs(input_file, ".mp4");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("mp4")
    .arg("--mp4-codec")
    .arg("libx265")
    .arg("--mp4-crf")
    .arg("4")
    .arg("--mp4-preset")
    .arg("fast")
    .arg("--mp4-pixel-format")
    .arg("yuv422p12")
    .arg("--mp4-frame-rate")
    .arg("2")
    .assert()
    .success()
    .stdout(format!("Writing \"{0}\" …\n", to_native_path(&output_file),));

  assert_eq!(
    get_video_stream_details(&output_file),
    Ok(VideoStreamDetails {
      codec_name: "hevc".to_string(),
      profile: "Rext".to_string(),
      width: 510,
      height: 510,
      pix_fmt: "yuv422p12le".to_string(),
      r_frame_rate: "2/1".to_string(),
    })
  );

  assert_eq!(get_video_frame_count(&output_file), Ok(3));
}

#[test]
fn render_overlays_and_rotate90() {
  let input_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png16")
    .arg("--overlays")
    .arg("--transform")
    .arg("rotate90")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(output_file, "render_overlays_and_rotate90.png");
}

#[test]
fn render_overlays_and_flip_horizontal() {
  let input_file =
    "../../../test/assets/pydicom/test_files/examples_overlay.dcm";
  let (output_file, output_directory) =
    prepare_outputs(input_file, ".0000.png");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
    .arg("-f")
    .arg("png16")
    .arg("--overlays")
    .arg("--transform")
    .arg("flip-horizontal")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", to_native_path(&output_file)));

  assert_image_snapshot!(
    output_file,
    "render_overlays_and_flip_horizontal.png"
  );
}

#[test]
fn with_output_directory() {
  let input_file = "../../../test/assets/other/mr_brucker_with_unaligned_multiframe_overlay.dcm";

  let output_directory = create_temp_dir();

  let output_files: Vec<String> = (0..4)
    .map(|i| {
      format!(
        "{}/mr_brucker_with_unaligned_multiframe_overlay.dcm.000{}.bin",
        output_directory.path().display(),
        i
      )
    })
    .collect();

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(output_directory.path())
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
}

#[test]
fn with_selected_frames() {
  let input_file = "../../../test/assets/pydicom/test_files/rtdose.dcm";
  let (output_file, output_directory) = prepare_outputs(input_file, "");

  let test_cases = [
    ("0", vec![0]),
    ("2", vec![2]),
    ("-4", vec![11]),
    ("3..5", vec![3, 4, 5]),
    ("12..", vec![12, 13, 14]),
    ("-9..-7", vec![6, 7, 8]),
  ];

  for (select_frames, expected_frames) in test_cases {
    let expected_output = expected_frames
      .iter()
      .map(|f| {
        format!(
          "Writing \"{}.{:04}.bin\" …\n",
          to_native_path(&output_file),
          f
        )
      })
      .collect::<String>();

    dcmfx_cli()
      .arg("get-pixel-data")
      .arg(input_file)
      .arg("--output-directory")
      .arg(output_directory.path())
      .arg(format!("--select-frames={}", select_frames))
      .assert()
      .success()
      .stdout(expected_output);
  }
}

#[tokio::test]
#[ignore]
async fn with_s3_input_and_output() {
  let input_file = "s3://dcmfx-test/other/monochrome_jpeg_xl.dcm";
  let output_directory =
    format!("with_s3_input_and_output/{}", rand::random::<u64>());
  let output_key =
    format!("{output_directory}/monochrome_jpeg_xl.dcm.0000.png");
  let output_path = format!("s3://dcmfx-test/{output_key}");

  dcmfx_cli()
    .arg("get-pixel-data")
    .arg(input_file)
    .arg("--output-directory")
    .arg(format!("s3://dcmfx-test/{}", output_directory))
    .arg("-f")
    .arg("png")
    .assert()
    .success()
    .stdout(format!("Writing \"{}\" …\n", output_path));

  let output_file = s3_get_object(&output_key).await;

  assert_image_snapshot!(output_file.path(), "jpeg_xl_monochrome_to_png.png");
}

/// For a given input file, returns a newly created temporary output directory
/// and the path to the output file in that directory for the input file.
///
/// This ensures test outputs don't conflict when run in parallel.
///
fn prepare_outputs<P: AsRef<std::path::Path>>(
  input_file: P,
  output_file_suffix: &str,
) -> (String, TempDir) {
  let output_directory = utils::create_temp_dir();

  let output_file = format!(
    "{}{}{}{}",
    output_directory.path().display(),
    std::path::MAIN_SEPARATOR,
    input_file.as_ref().file_name().unwrap().display(),
    output_file_suffix
  );

  (output_file, output_directory)
}

/// Returns the number of frames in the specified video file.
///
fn get_video_frame_count(path: &str) -> Result<u32, String> {
  let output = Command::new("ffprobe")
    .args([
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-count_frames",
      "-show_entries",
      "stream=nb_read_frames",
      "-of",
      "default=nokey=1:noprint_wrappers=1",
      &path,
    ])
    .output()
    .map_err(|e| e.to_string())?;

  String::from_utf8_lossy(&output.stdout)
    .trim()
    .parse::<u32>()
    .map_err(|e| e.to_string())
}

/// Returns details on the video stream of a video file.
///
fn get_video_stream_details(path: &str) -> Result<VideoStreamDetails, String> {
  let output = Command::new("ffprobe")
    .args([
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=codec_name,profile,width,height,pix_fmt,nb_frames,r_frame_rate",
      "-of",
      "json",
      &path,
    ])
    .output()
    .map_err(|e| e.to_string())?;

  let mut parsed: FfprobeOutput =
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

  Ok(parsed.streams.remove(0))
}

#[derive(Debug, serde::Deserialize)]
struct FfprobeOutput {
  streams: Vec<VideoStreamDetails>,
}

#[derive(Debug, serde::Deserialize, PartialEq)]
pub struct VideoStreamDetails {
  pub codec_name: String,
  pub profile: String,
  pub width: i32,
  pub height: i32,
  pub pix_fmt: String,
  pub r_frame_rate: String,
}
