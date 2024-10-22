::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

# `dcmfx_json`

This library converts between DICOM data and the DICOM JSON Model. It supports
streaming conversion of DICOM Part 10 data to DICOM JSON.

## Details

1. This library optionally extends the DICOM JSON specification to allow
   encapsulated pixel data to be stored. It does this by encoding the binary
   data present in the '(7FE0,0010) PixelData' data element in Base64. This
   matches the behavior of other libraries such as
   [`pydicom`](https://github.com/pydicom/pydicom)

2. The `BulkDataURI` used to store and retrieve data from external sources is
   not supported. Binary data must be encoded inline using Base64. If
   `BulkDataURI` is encountered then an error will be returned.

3. Floating point `Infinity`, `-Infinity`, and `NaN` are supported by the DICOM
   P10 format but are not supported by JSON's `number` type. As a workaround,
   such values are stored as quoted strings: `"Infinity"`, `"-Infinity"`, and
   `"NaN"`. Such non-finite values are rare in DICOM.

4. 64-bit integer values outside the range representable by JavaScript's
   `number` type are stored as quoted strings to avoid loss of precision.

## Usage

The following code reads a DICOM P10 file, converts it to DICOM JSON, and prints
the result to stdout. It then converts the DICOM JSON string back into a data
set and prints that to stdout.

### Gleam

```sh
gleam add dcmfx_core dcmfx_json dcmfx_p10
```

```gleam
import dcmfx_core/data_set
import dcmfx_json
import dcmfx_json/json_config.{DicomJsonConfig}
import dcmfx_p10
import gleam/io

pub fn main() {
  // Read input file
  let dicom_file = "input.dcm"
  let assert Ok(ds) = dcmfx_p10.read_file(dicom_file)

  // Convert data set to JSON and print to stdout
  let json_config = DicomJsonConfig(store_encapsulated_pixel_data: True)
  let assert Ok(ds_json) = dcmfx_json.data_set_to_json(ds, json_config)
  io.println(ds_json)

  // Convert JSON back to a data set and print to stdout
  let assert Ok(new_ds) = dcmfx_json.json_to_data_set(ds_json)
  data_set.print(new_ds)
}
```

### Rust

```sh
cargo add dcmfx
```

```rust
use dcmfx::core::*;
use dcmfx::json::*;
use dcmfx::p10::*;

pub fn main() {
  // Read input file
  let dicom_file = "input.dcm";
  let ds = DataSet::read_p10_file(dicom_file).unwrap();

  // Convert data set to JSON and print to stdout
  let json_config = DicomJsonConfig {
    store_encapsulated_pixel_data: true
  };
  let ds_json = ds.to_json(json_config).unwrap();
  println!("{}", ds_json);

  // Convert JSON back to a data set and print to stdout
  let new_ds = DataSet::from_json(&ds_json).unwrap();
  new_ds.print(None);
}
```
