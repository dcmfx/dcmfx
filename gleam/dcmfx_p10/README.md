# DCMfx - Part 10 Library

Reads and writes the DICOM Part 10 (P10) binary format used to store and
transmit DICOM-based medical imaging information.

Part of the DCMfx project.

[![Package Version](https://img.shields.io/hexpm/v/dcmfx_p10)](https://hex.pm/packages/dcmfx_p10)
[![Hex Docs](https://img.shields.io/badge/hex-docs-ffaff3)](https://hexdocs.pm/dcmfx_p10/)
![Erlang Compatible](https://img.shields.io/badge/target-erlang-a90432)
![JavaScript Compatible](https://img.shields.io/badge/target-javascript-f3e155)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPLv3-blue.svg)](https://www.gnu.org/licenses/agpl-3.0)

## Usage

Add this library to your Gleam project with the following command:

```sh
gleam add dcmfx_core dcmfx_p10
```

The following code reads a DICOM P10 file, prints it to stdout, and then writes
it out to a new DICOM P10 file.

```gleam
import dcmfx_core/data_set
import dcmfx_p10

fn dcmfx_p10_example(filename: String) {
  let assert Ok(ds) = dcmfx_p10.read_file(filename)

  // Print the data set to stdout
  data_set.print(ds)

  // Write the data set to a new DICOM P10 file
  let assert Ok(Nil) = dcmfx_p10.write_file(filename <> ".new.dcm", ds)
}
```

The above example code loads the entire DICOM P10 file into an in-memory data
set, however this library also provides DICOM P10 streaming capabilities that
can be used to read, modify, and write DICOM P10 data without loading whole data
sets into memory.

Streaming DICOM P10 data can be modified in the following ways:

1. Specific data elements, whether individual values or nested sequences, can be
   removed based on a condition. These filtered data elements can optionally be
   turned into a data set, allowing specific data elements to be extracted from
   a stream as it passes through.

2. New data elements, both individual values and sequences, can be inserted
   into the main data set, replacing existing data elements in the stream if
   present.

API documentation is available [https://hexdocs.pm/dcmfx_p10](here).

## Conformance

This library is compatible with all valid DICOM P10 data and does not require
input data to strictly conform to the DICOM P10 standard. Retired transfer
syntaxes as of DICOM PS3.5 2024c are not supported, with the exception of
'Explicit VR Big Endian'.

When writing DICOM P10 content, strict conformance of the data being written is
not enforced. The reason is that any DICOM P10 data that was able to be _read_
should also be able to be _written_, even if parts of it were non-conformant in
some way. The `dcmfx_core/data_element_value` module provides constructor
functions for creating new data element values, and these _do_ enforce strict
conformance.

## Limitations

### UTF-8 Conversion

This library converts all strings contained in DICOM P10 data to UTF-8 as part
of the read process. This is done because native DICOM string data is complex to
work with and UTF-8 is the preferred string encoding of modern systems.

DICOM P10 data written by this library therefore always uses UTF-8. Note that
text encoded in UTF-8 may consume more bytes than other encodings for some
languages.

### Sequences and Items of Undefined Length

This library converts sequences and items that have defined lengths to use
undefined lengths with explicit delimiters. This consumes slightly more space,
particularly for data sets that have a large number of sequences or items, but
is necessary in order to be able to stream DICOM P10 data in a memory-efficient
way.

## License

DCMfx is published under the GNU Affero General Public License Version 3
(AGPLv3). This license permits commercial use; however, any software that
incorporates DCMfx, either directly or indirectly, must also be released under
the AGPLv3 or a compatible license. This includes making the source code of the
combined work available under the same terms, and ensuring that users who
interact with the software over a network can access the source code.

Copyright © Dr Richard Viney, 2024.
