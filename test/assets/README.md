# DCMfx - Test Assets

This folder contains assets used by the integration tests for the DCMfx
libraries. These test many different types of DICOM files and check the result
against a known-correct piece of output, e.g. JSON.

Most files were sourced from the test suites used by other DICOM
implementations, specifically `pydicom` and `fo-dicom`. A small number came from
other publicly available sources.

Known-correct JSON is generated with `pydicom` using the `dicoms_to_json.py`
script. Note that in some cases issues in `pydicom` cause its JSON output to be
different to DCMfx, and this is corrected for by this script.

## Excluded Test Files

Some test files were excluded. The reasons for these exclusions are listed here:

- `fo-dicom/GH340.dcm`, `pydicom/test_files/badVR.dcm`, `fo-dicom/GH195.dcm`:
  excluded because they contain an Integer/Decimal String (`IS` / `DS`) with an
  invalid value (`"01160501010100"`, `"1A"`, and `"1.9.10.0"` respectively) that
  can't be stored in DICOM JSON. Note that DCMTK's `dcm2json` handles this case
  by storing the invalid value as a string, but this isn't valid in DICOM JSON.

- `pydicom/test_files/ExplVR_BigEndNoMeta.dcm`,
  `pydicom/test_files/ExplVR_LitEndNoMeta.dcm`: excluded because DCMfx uses the
  'Implicit VR Little Endian' transfer syntax when no other is specified.

- `fo-dicom/GH1339.dcm`, `pydicom/test_files/MR_truncated.dcm`,
  `pydicom/test_files/rtplan_truncated.dcm`: excluded because DCMfx doesn't
  silently succeed on arbitrarily truncated files. Only files truncated exactly
  on data element boundaries are supported by DCMfx. It's straightforward to
  load such files with DCMfx if desired, e.g. for recovering damaged data, but
  for safety reasons it isn't the default behavior.

- `pydicom/test_files/SC_rgb_jpeg.dcm`: excluded because DCMfx doesn't attempt
  to read invalid explicit VR data as implicit VR as an error recovery method.

- `pydicom/palettes/winter.dcm`: excluded because DCMfx does not accept DICOM
  P10 data containing data elements that are not in ascending order.

- `fo-dicom/GH1301.dcm`: excluded because DCMfx converts invalid bytes in a Code
  String (`CS`) value to a `?` character, but `pydicom` interprets them as ISO
  8859-1 bytes.

- `fo-dicom/GH1376.dcm`: excluded because DCMfx does not allow File Meta
  Information data elements (those with group = 0x0002) in the main data set.

- `fo-dicom/GH179A.dcm`, `fo-dicom/GH179B.dcm`, `fo-dicom/GH626.dcm`: excluded
  because `pydicom` reads these invalid files incorrectly.

- `fo-dicom/DIRW0007.dcm`, `fo-dicom/GH364.dcm`, `fo-dicom/FreezePattern.dcm`,
  `fo-dicom/GH177_D_CLUNIE_CT1_IVRLE_BigEndian_undefined_length.dcm`: excluded
  because `pydicom` errors reading these files.

- `fo-dicom/10200904.dcm`, `fo-dicom/ETIAM_video_002.dcm`,
  `fo-dicom/GH1049_planar_0.dcm`, `fo-dicom/GH1049_planar_1.dcm`,
  `fo-dicom/GH133.dcm`, `fo-dicom/GH1442.dcm`, `fo-dicom/GH1728.dcm`,
  `fo-dicom/GH645.dcm`, `fo-dicom/MOVEKNEE.dcm`, `fo-dicom/test_720.dcm`,
  `fo-dicom/VL_Olympus1.dcm`, `fo-dicom/VL5_J2KI.dcm`: excluded to avoid
  including large files in the repository.

- `fo-dicom/GH227.dcm`, `fo-dicom/test_SR.dcm`: excluded because they are exact
  duplicates of files in pydicom's test suite.
