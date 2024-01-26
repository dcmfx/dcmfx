# DCMfx - JSON Library

Converts DICOM data sets to and from DICOM JSON.

Part of the DCMfx project.

[![Package Version](https://img.shields.io/hexpm/v/dcmfx_json)](https://hex.pm/packages/dcmfx_json)
[![Hex Docs](https://img.shields.io/badge/hex-docs-ffaff3)](https://hexdocs.pm/dcmfx_json/)
![Erlang Compatible](https://img.shields.io/badge/target-erlang-a90432)
![JavaScript Compatible](https://img.shields.io/badge/target-javascript-f3e155)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Usage

Add this library to your Gleam project with the following command:

```sh
gleam add dcmfx_core dcmfx_json dcmfx_p10
```

The following code reads a DICOM P10 file, converts it to DICOM JSON, and prints
the result to stdout. It then converts the DICOM JSON string back into a data
set and prints that to stdout.

```gleam
import dcmfx_core/data_set
import dcmfx_json
import dcmfx_p10
import gleam/io

fn dcmfx_json_example(dicom_file: String) {
  let assert Ok(ds) = dcmfx_p10.read_file(dicom_file)
  let assert Ok(ds_json) = dcmfx_json.data_set_to_json(ds)

  io.println(ds_json)

  let assert Ok(new_ds) = dcmfx_json.json_to_data_set(ds_json)
  data_set.print(new_ds)
}
```

API documentation is available [https://hexdocs.pm/dcmfx_json](here).

## Conformance

1. This library optionally extends the DICOM JSON specification to allow
   encapsulated pixel data to be stored. It does this by encoding the binary
   data present in the '(7FE0,0010) PixelData' data element in Base64. This
   matches the behavior of other libraries such as
   [`pydicom`](https://github.com/pydicom/pydicom)

2. This library does not support the `BulkDataURI` property to store and
   retrieve data from external sources. Binary data must be encoded inline using
   Base64.

3. Floating point `Infinity`, `-Infinity`, and `NaN` are supported by the DICOM
   P10 format but are not supported by JSON's `number` type. As a workaround, such
   values are stored as quoted strings: `"Infinity"`, `"-Infinity"`, and
   `"NaN"`. Non-finite values are exceedingly rare in DICOM.

4. 64-bit integer values outside the range representable by JavaScript's
   `number` type are stored as quoted strings to avoid loss of precision.

## License

DCMfx is published under the GNU Affero General Public License Version 3
(AGPLv3). This license permits commercial use; however, any software that
incorporates DCMfx, either directly or indirectly, must also be released under
the AGPLv3 or a compatible license. This includes making the source code of the
combined work available under the same terms, and ensuring that users who
interact with the software over a network can access the source code.

Copyright © Dr Richard Viney, 2024.
