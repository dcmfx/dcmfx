# DICOM Libraries

The internal libraries that make up DCMfx are:

- **`dcmfx_core` / `dcmfx::core`**. Provides core DICOM concepts such as data
  sets, data elements, value representations, value multiplicity, transfer
  syntaxes, and a dictionary of the data elements defined in DICOM PS3.6 as well
  as well-known private data elements.

- **`dcmfx_p10` / `dcmfx::p10`**. Reads, writes, and modifies the DICOM P10 file
  format. Uses a streaming design suited for highly concurrent and
  memory-constrained environments. Provides transforms for reading and modifying
  streams of DICOM P10 data.

- **`dcmfx_json` / `dcmfx::json`**. Converts between DICOM data sets and the
  DICOM JSON Model, with stream conversion to DICOM JSON. Optionally extends the
  DICOM JSON Model to allow for the storage of encapsulated pixel data.

- **`dcmfx_pixel_data` / `dcmfx::pixel_data`**. Extracts frames of pixel data
  from a DICOM data set and streams of DICOM P10 data. The Rust version also
  supports decoding and encoding pixel data.

- **`dcmfx_anonymize` / `dcmfx::anonymize`**. Anonymizes the data elements in a
  DICOM data set or stream of DICOM P10 data.

See the [examples](./examples/) section for code examples showing how to perform
common tasks using the DCMfx libraries.

## Languages

DCMfx is dual-implemented in two languages: [Gleam](https://gleam.run) and
[Rust](https://rust-lang.org). The two implementations have identical designs
and very similar APIs.

### Gleam

The Gleam implementation allows DCMfx to be used directly from Gleam, Elixir,
Erlang, JavaScript, and TypeScript. It's also the only DICOM library that runs
natively on the BEAM VM.

### Rust

The Rust implementation allows DCMfx to be used from Rust, compile to WASM, and
is faster with lower memory usage. The Rust implementation is used for the
[CLI tool](../tools/cli).
