::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

# `dcmfx_p10`

This library reads, writes, and modifies the DICOM Part 10 (P10) file format.
It uses a streaming design that makes it suited for highly concurrent and
memory-constrained environments, as whole data sets don't need to be held fully
in memory.

It also provides transforms for modifying a stream of DICOM P10 data. Streaming
DICOM P10 data can be modified in the following ways:

1. Specific data elements, whether individual values or nested sequences, can be
   removed based on a condition. These filtered data elements can optionally be
   turned into a data set, allowing specific data elements to be extracted from
   a stream as it passes through.

2. New data elements, both individual values and sequences, can be inserted
   into the root data set, replacing existing data elements in the stream if
   present.

## Conformance

This library is compatible with all valid DICOM P10 data and does not require
input data to strictly conform to the DICOM P10 standard. Retired transfer
syntaxes as of DICOM PS3.5 2024c are not supported, with the exception of
'Explicit VR Big Endian'.

When writing DICOM P10 data, strict conformance of the data being written is not
enforced. The reason is that any DICOM P10 data that was able to be _read_
should also be able to be _written_, even if parts of it were non-conformant in
some way. Note that when creating new data element values from scratch in
`dcmfx_core` strict conformance is enforced by default.

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

## Usage

The following code reads a DICOM P10 file, prints it to stdout, and then writes
it out to a new DICOM P10 file.

### Gleam

```sh
gleam add dcmfx_core dcmfx_p10
```

```gleam
import dcmfx_core/data_set
import dcmfx_p10

pub fn main() {
  let dicom_file = "input.dcm"
  let assert Ok(ds) = dcmfx_p10.read_file(filename)

  // Print the data set to stdout
  data_set.print(ds)

  // Write the data set to a new DICOM P10 file
  let new_dicom_file = filename <> ".new.dcm"
  let assert Ok(Nil) = dcmfx_p10.write_file(new_dicom_file, ds)
}
```

### Rust

```sh
cargo add dcmfx
```

```rust
use dcmfx::core::*;
use dcmfx::p10::*;

pub fn main() {
  let dicom_file = "input.dcm";
  let ds = DataSet::read_p10_file(dicom_file).unwrap();

  // Print the data set to stdout
  ds.print(None);

  // Write the data set to a new DICOM P10 file
  let new_dicom_file = format!("{dicom_file}.new.dcm");
  ds.write_p10_file(&new_dicom_file, None).unwrap();
}
```
