# CLI Tool

The `dcmfx` CLI tool makes the capabilities of DCMfx available on the command
line.

## Installation

Download the latest version for your platform as a standalone binary
[here](https://github.com/dcmfx/dcmfx/releases/latest), or install via
[Homebrew](https://brew.sh) on macOS and Linux:

```sh
brew tap dcmfx/tap
brew install dcmfx
```

## Usage

After installation, run `dcmfx --help` to see the available commands:

```
$ dcmfx --help

DCMfx is a CLI app for working with DICOM and DICOM JSON

Usage: dcmfx [OPTIONS] <COMMAND>

Commands:
  extract-pixel-data  Extracts the pixel data from a DICOM P10 file and writes
                      each frame to a separate image file
  modify              Reads a DICOM P10 file, applies requested modifications,
                      and writes out a new DICOM P10 file
  print               Prints the content of a DICOM P10 file
  to-dcm              Converts a DICOM JSON file to a DICOM P10 file
  to-json             Converts a DICOM P10 file to a DICOM JSON file
  help                Print this message or the help of the given subcommand(s)

Options:
      --print-stats  Write timing and memory stats to stderr on exit
  -h, --help         Print help
  -V, --version      Print version
```

### Examples

1. Print a DICOM P10 file's data set to stdout:

   ```sh
   dcmfx print input.dcm
   ```

2. Convert a DICOM P10 file to a DICOM JSON file:

   ```sh
   dcmfx to-json input.dcm output.json
   ```

   To pretty-print the DICOM JSON directly to stdout:

   ```sh
   dcmfx to-json --pretty input.dcm -
   ```

3. Convert a DICOM JSON file to a DICOM P10 file:

   ```sh
   dcmfx to-dcm input.json output.dcm
   ```

4. Extract pixel data from a DICOM P10 file to image files:

   ```sh
   dcmfx extract-pixel-data input.dcm
   ```

5. Rewrite a DICOM P10 file. This will convert the specific character set to
   UTF-8, change sequences and items to undefined length, and correct certain
   invalid file errors:

   ```sh
   dcmfx modify input.dcm output.dcm
   ```

6. Modify a DICOM P10 file to use the 'Deflated Explicit VR Little Endian'
   transfer syntax with maximum compression:

   ```sh
   dcmfx modify input.dcm output.dcm \
     --transfer-syntax deflated-explicit-vr-little-endian \
     --zlib-compression-level 9
   ```

   The `modify` command can only convert between the following transfer
   syntaxes:

   - Implicit VR Little Endian
   - Explicit VR Little Endian
   - Deflated Explicit VR Little Endian
   - Explicit VR Big Endian

   Conversion between other transfer syntaxes is not supported.

7. Anonymize a DICOM P10 file to have all identifying data elements removed:

   ```sh
   dcmfx modify input.dcm output.dcm --anonymize
   ```

   Note that this does not remove any identifying information baked into pixel
   data or other binary data elements.

8. Remove the top-level *'(7FE0,0010) Pixel Data'* data element from a DICOM P10
   file:

   ```sh
   dcmfx modify input.dcm output.dcm --delete-tags 7FE00010
   ```

   Multiple data elements can be deleted by using a comma as a separator:

   ```sh
   dcmfx modify input.dcm output.dcm --delete-tags 00100010,00100030
   ```

## Gleam CLI

The above examples assume the Rust version of the CLI tool is in use, however
all commands are also supported by the Gleam version of the CLI, which can be
run with `gleam run` in `/src/gleam/dcmfx_cli`.

The Gleam CLI is primarily used when working on the Gleam implementation of
DCMfx. The Rust CLI is recommended for regular use as it is faster, has a few
additional features, and is a single standalone binary.
