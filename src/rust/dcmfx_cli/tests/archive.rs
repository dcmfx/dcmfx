mod utils;

use std::path::Path;

use insta::assert_snapshot;
use utils::{create_temp_dir, dcmfx_cli, get_stderr, get_stdout};

/// The DICOM files that are archived by these tests. The three `TestPattern`
/// files are all in the same series, and the `CT_small` file is for a different
/// patient.
///
const DICOM_FILES: [&str; 4] = [
  "../../../test/assets/fo-dicom/TestPattern_RGB.dcm",
  "../../../test/assets/fo-dicom/TestPattern_Palette.dcm",
  "../../../test/assets/fo-dicom/TestPattern_Palette_16.dcm",
  "../../../test/assets/pydicom/test_files/CT_small.dcm",
];

/// Filters for values in a DICOMDIR that aren't stable across runs or DCMfx
/// versions, and so aren't able to be included in a snapshot.
///
fn snapshot_filters() -> Vec<(&'static str, &'static str)> {
  vec![
    // The Implementation Version Name contains the current DCMfx version
    (
      r#"Implementation Version Name +\[ *\d+ bytes\] "DCMfx[^\n]*"#,
      "Implementation Version Name          [    14 bytes] \"DCMFX_VERSION\"",
    ),
    // A new Media Storage SOP Instance UID is generated for each DICOMDIR
    (r"1\.2\.826\.0\.1\.3680043\.10\.1462\.2\.\d{30,}", "[UID]"),
  ]
}

#[tokio::test]
async fn with_multiple_inputs() {
  let output_dir = create_temp_dir();
  let output_filename = output_dir.path().join("archive.zip");

  let mut command = dcmfx_cli();
  command.arg("archive");
  command.args(DICOM_FILES);
  command
    .arg("--file-set-id")
    .arg("DCMFX_TEST")
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .success();

  let entries = read_zip_entries(&output_filename).await;

  // Check the archive contains an entry for each input DICOM, followed by the
  // DICOMDIR
  assert_eq!(
    entries
      .iter()
      .map(|entry| entry.0.as_str())
      .collect::<Vec<_>>(),
    [
      "DICOM/IM000000",
      "DICOM/IM000001",
      "DICOM/IM000002",
      "DICOM/IM000003",
      "DICOMDIR"
    ]
  );

  // Check the archived DICOMs are byte-for-byte identical to the inputs
  for (index, dicom_file) in DICOM_FILES.iter().enumerate() {
    assert_eq!(entries[index].1, std::fs::read(dicom_file).unwrap());
  }

  // Check the content of the DICOMDIR. Note that the instance records are
  // ordered by their SOP Instance UID, which isn't the order the DICOMs were
  // specified in.
  let stdout = print_dicom_file(output_dir.path(), &entries[4].1, None);
  insta::with_settings!({filters => snapshot_filters()}, {
    assert_snapshot!("with_multiple_inputs", stdout);
  });
}

#[tokio::test]
async fn with_deflate_compression() {
  let output_dir = create_temp_dir();
  let output_filename = output_dir.path().join("archive.zip");

  let mut command = dcmfx_cli();
  command.arg("archive");
  command.args(DICOM_FILES);
  command
    .arg("--zip-compression-method")
    .arg("deflate")
    .arg("--deflate-compression-level")
    .arg("maximum")
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .success();

  let entries = read_zip_entries(&output_filename).await;

  assert_eq!(
    entries
      .iter()
      .map(|entry| entry.0.as_str())
      .collect::<Vec<_>>(),
    [
      "DICOM/IM000000",
      "DICOM/IM000001",
      "DICOM/IM000002",
      "DICOM/IM000003",
      "DICOMDIR"
    ]
  );

  // Check the archived DICOMs decompress back to the exact input data
  for (index, dicom_file) in DICOM_FILES.iter().enumerate() {
    assert_eq!(entries[index].1, std::fs::read(dicom_file).unwrap());
  }

  // Check the archive is smaller than the DICOMs it contains
  let uncompressed_size = DICOM_FILES
    .iter()
    .map(|dicom_file| std::fs::metadata(dicom_file).unwrap().len())
    .sum::<u64>();
  assert!(
    std::fs::metadata(&output_filename).unwrap().len() < uncompressed_size
  );
}

/// A DICOM that has a person name containing non-ASCII characters, i.e. one
/// that uses an extended character set.
///
const UTF8_DICOM_FILE: &str = "../../../test/assets/other/vr_2022.dcm";

#[tokio::test]
async fn with_utf8_person_name() {
  let output_dir = create_temp_dir();
  let output_filename = output_dir.path().join("archive.zip");

  dcmfx_cli()
    .arg("archive")
    .arg(UTF8_DICOM_FILE)
    .arg(DICOM_FILES[3])
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .success();

  let entries = read_zip_entries(&output_filename).await;

  assert_eq!(
    entries
      .iter()
      .map(|entry| entry.0.as_str())
      .collect::<Vec<_>>(),
    ["DICOM/IM000000", "DICOM/IM000001", "DICOMDIR"]
  );
  assert_eq!(entries[0].1, std::fs::read(UTF8_DICOM_FILE).unwrap());

  // Check the DICOMDIR specifies UTF-8 as the character set of the directory
  // record containing the person name, and that the name is intact. The other
  // patient's records use only ASCII values, so don't specify a character set.
  let stdout = print_dicom_file(output_dir.path(), &entries[2].1, Some(200));
  assert_eq!(stdout.matches("ISO_IR 192").count(), 1);
  assert!(stdout.contains("Äneas^Rüdiger"));

  insta::with_settings!({filters => snapshot_filters()}, {
    assert_snapshot!("with_utf8_person_name", stdout);
  });
}

#[tokio::test]
async fn with_file_extension() {
  // The leading period on the file extension is optional
  for extension in ["dcm", ".dcm"] {
    let output_dir = create_temp_dir();
    let output_filename = output_dir.path().join("archive.zip");

    let mut command = dcmfx_cli();
    command.arg("archive");
    command.args(&DICOM_FILES[..2]);
    command
      .arg("--extension")
      .arg(extension)
      .arg("--output-filename")
      .arg(&output_filename)
      .assert()
      .success();

    let entries = read_zip_entries(&output_filename).await;

    assert_eq!(
      entries
        .iter()
        .map(|entry| entry.0.as_str())
        .collect::<Vec<_>>(),
      ["DICOM/IM000000.dcm", "DICOM/IM000001.dcm", "DICOMDIR"]
    );

    // Check the DICOMDIR's referenced file IDs match the names in the ZIP file
    let stdout = print_dicom_file(output_dir.path(), &entries[2].1, Some(200));
    assert!(stdout.contains(r#""IM000000.dcm""#));
    assert!(stdout.contains(r#""IM000001.dcm""#));
  }
}

#[tokio::test]
async fn with_path_structure() {
  // The first three DICOMs are in one patient's study and series, and the
  // fourth is for a different patient
  let expected_file_names = [
    (
      "flat",
      vec![
        "DICOM/IM000000",
        "DICOM/IM000001",
        "DICOM/IM000002",
        "DICOM/IM000003",
      ],
    ),
    (
      "study",
      vec![
        "DICOM/ST000000/IM000000",
        "DICOM/ST000000/IM000001",
        "DICOM/ST000000/IM000002",
        "DICOM/ST000001/IM000000",
      ],
    ),
    (
      "study-series",
      vec![
        "DICOM/ST000000/SE000000/IM000000",
        "DICOM/ST000000/SE000000/IM000001",
        "DICOM/ST000000/SE000000/IM000002",
        "DICOM/ST000001/SE000001/IM000000",
      ],
    ),
    (
      "patient-study-series",
      vec![
        "DICOM/PA000000/ST000000/SE000000/IM000000",
        "DICOM/PA000000/ST000000/SE000000/IM000001",
        "DICOM/PA000000/ST000000/SE000000/IM000002",
        "DICOM/PA000001/ST000001/SE000001/IM000000",
      ],
    ),
  ];

  for (path_structure, expected_file_names) in expected_file_names {
    let output_dir = create_temp_dir();
    let output_filename = output_dir.path().join("archive.zip");

    let mut command = dcmfx_cli();
    command.arg("archive");
    command.args(DICOM_FILES);
    command
      .arg("--path-structure")
      .arg(path_structure)
      .arg("--output-filename")
      .arg(&output_filename)
      .assert()
      .success();

    let entries = read_zip_entries(&output_filename).await;

    assert_eq!(
      entries
        .iter()
        .map(|entry| entry.0.as_str())
        .collect::<Vec<_>>(),
      [expected_file_names.as_slice(), &["DICOMDIR"]].concat()
    );

    // Check the archived DICOMs are byte-for-byte identical to the inputs
    for (index, dicom_file) in DICOM_FILES.iter().enumerate() {
      assert_eq!(entries[index].1, std::fs::read(dicom_file).unwrap());
    }
  }
}

#[tokio::test]
async fn with_invalid_input() {
  let output_dir = create_temp_dir();
  let output_filename = output_dir.path().join("archive.zip");

  let invalid_input = output_dir.path().join("invalid.dcm");
  std::fs::write(&invalid_input, vec![0u8; 1024]).unwrap();

  // An input that's too short to contain the 'DICM' prefix is also an error
  let short_input = output_dir.path().join("short.dcm");
  std::fs::write(&short_input, b"DICM").unwrap();

  let assert = dcmfx_cli()
    .arg("archive")
    .arg(&short_input)
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .failure();

  assert!(get_stderr(assert).contains("'DICM' prefix is not present"));

  // An input that isn't DICOM P10 data is an error, and no archive is written
  let assert = dcmfx_cli()
    .arg("archive")
    .arg(DICOM_FILES[0])
    .arg(&invalid_input)
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .failure();

  assert!(get_stderr(assert).contains("'DICM' prefix is not present"));
  assert!(!output_filename.exists());

  // Such inputs are skipped when --ignore-non-dicom-inputs is specified
  dcmfx_cli()
    .arg("archive")
    .arg(DICOM_FILES[0])
    .arg(&invalid_input)
    .arg("--ignore-non-dicom-inputs")
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .success();

  let entries = read_zip_entries(&output_filename).await;

  assert_eq!(
    entries
      .iter()
      .map(|entry| entry.0.as_str())
      .collect::<Vec<_>>(),
    ["DICOM/IM000000", "DICOMDIR"]
  );
  assert_eq!(entries[0].1, std::fs::read(DICOM_FILES[0]).unwrap());
}

#[test]
fn with_overwrite() {
  let output_dir = create_temp_dir();
  let output_filename = output_dir.path().join("archive.zip");
  std::fs::write(&output_filename, "").unwrap();

  dcmfx_cli()
    .arg("archive")
    .arg(DICOM_FILES[0])
    .arg("--output-filename")
    .arg(&output_filename)
    .assert()
    .failure();

  dcmfx_cli()
    .arg("archive")
    .arg(DICOM_FILES[0])
    .arg("--output-filename")
    .arg(&output_filename)
    .arg("--overwrite")
    .assert()
    .success();
}

/// Reads all of the entries in a ZIP file, returning their filenames and
/// decompressed data. The CRC of each entry is checked as it's read.
///
async fn read_zip_entries(path: &Path) -> Vec<(String, Vec<u8>)> {
  use async_zip::base::read::mem::ZipFileReader;

  let reader = ZipFileReader::new(std::fs::read(path).unwrap())
    .await
    .unwrap();

  let filenames = reader
    .file()
    .entries()
    .iter()
    .map(|entry| entry.filename().as_str().unwrap().to_string())
    .collect::<Vec<String>>();

  let mut entries = vec![];

  for (index, filename) in filenames.into_iter().enumerate() {
    let mut data = vec![];
    reader
      .reader_with_entry(index)
      .await
      .unwrap()
      .read_to_end_checked(&mut data)
      .await
      .unwrap();

    entries.push((filename, data));
  }

  entries
}

/// Writes DICOM P10 data to a file and returns the output of running the
/// `print` command on it.
///
fn print_dicom_file(
  dir: &Path,
  dicom_p10: &[u8],
  max_width: Option<usize>,
) -> String {
  let path = dir.join("print_input.dcm");
  std::fs::write(&path, dicom_p10).unwrap();

  let mut command = dcmfx_cli();
  command.arg("print");

  if let Some(max_width) = max_width {
    command.arg("--max-width").arg(max_width.to_string());
  }

  let assert = command.arg(&path).assert().success();

  get_stdout(assert)
}
