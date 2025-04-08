# CLI Tool

The `dcmfx` CLI tool makes the capabilities of DCMfx available on the command
line.

## Installation

Select your platform below to see its installation instructions.

::: details Windows
Download the latest version of `dcmfx.exe` for Windows [here](https://github.com/dcmfx/dcmfx/releases/latest).
:::

::: details macOS
Install [Homebrew](https://brew.sh), then install DCMfx using the provided tap:

```sh
brew tap dcmfx/tap
brew install dcmfx
```
:::

::: details Linux
- For Ubuntu, Debian, Linux Mint, and other Debian-based distributions, an APT
  repository is provided:

  ```sh
  echo "deb [trusted=yes] https://dcmfx.github.io/apt-repository stable main" | sudo tee /etc/apt/sources.list.d/dcmfx.list
  sudo apt update
  sudo apt install dcmfx
  ```

---

- For Red Hat, Fedora, Amazon Linux, and other RPM-based distributions, a YUM
  repository is provided:

  ```sh
  echo -e "[dcmfx]\nname=DCMfx\nbaseurl=http://dcmfx.github.io/yum-repository\nenabled=1\ngpgcheck=0" | sudo tee /etc/yum.repos.d/dcmfx.repo
  sudo yum makecache
  sudo yum install dcmfx
  ```

---

- For Arch Linux, a [package](https://aur.archlinux.org/packages/dcmfx) is
  provided in the Arch User Repository (AUR). Install it with your preferred
  AUR helper (e.g. [paru](https://github.com/Morganamilo/paru)):

  ```sh
  paru -S dcmfx
  ```

---

- Alternatively, download the latest binary or package
[here](https://github.com/dcmfx/dcmfx/releases/latest) and install it manually.
:::

## Usage

After installation, run `dcmfx --help` to see the available commands:

```
$ dcmfx --help

DCMfx is a CLI tool for working with DICOM and DICOM JSON

Usage: dcmfx [OPTIONS] <COMMAND>

Commands:
  get-pixel-data  Extracts pixel data from DICOM P10 files, writing it to image
                  and video files
  modify          Modifies the content of DICOM P10 files
  print           Prints the content of DICOM P10 files
  json-to-dcm     Converts DICOM JSON files to DICOM P10 files
  dcm-to-json     Converts DICOM P10 files to DICOM JSON files
  help            Print this message or the help of the given subcommand(s)

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
   dcmfx dcm-to-json input.dcm
   ```

   To pretty-print the DICOM JSON directly to stdout:

   ```sh
   dcmfx dcm-to-json input.dcm --pretty --output-filename -
   ```

3. Convert a DICOM JSON file to a DICOM P10 file:

   ```sh
   dcmfx json-to-dcm input.json
   ```

4. Extract pixel data from a DICOM P10 file to one image file per frame:

   ```sh
   dcmfx get-pixel-data input.dcm
   ```

   Each frame of pixel data can be decoded and written to PNG, 16-bit PNG, JPEG,
   or WebP Lossless images:

   ```sh
   dcmfx get-pixel-data input.dcm --format png
   dcmfx get-pixel-data input.dcm --format png16
   dcmfx get-pixel-data input.dcm --format jpg --jpeg-quality 90
   ```

   Single channel data can also specify a VOI window center and width and/or a
   well-known color palette to use:

   ```sh
   dcmfx get-pixel-data input.dcm --format png --voi-window 500 2000 \
     --color-palette hot-iron
   ```

5. Extract pixel data from a DICOM P10 file to an MP4 video:

   ```sh
   dcmfx get-pixel-data input.dcm --format mp4
   ```

   Additional options are available to specify the codec, video quality, encoder
   preset, pixel format, frame rate override, and so on.

6. Rewrite a DICOM P10 file. This will convert the specific character set to
   UTF-8, change sequences and items to undefined length, and correct invalid
   files where possible:

   ```sh
   dcmfx modify input.dcm --output-filename output.dcm
   ```

7. Modify a DICOM P10 file to use the 'Deflated Explicit VR Little Endian'
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

8. Anonymize a DICOM P10 file in-place by removing all identifying data
   elements and private data elements:

   ```sh
   dcmfx modify input.dcm --in-place --anonymize
   ```

   Note that this does not remove any identifying information baked into pixel
   data or other binary data elements.

9. Remove the top-level *'(7FE0,0010) Pixel Data'* data element from a DICOM P10
   file:

   ```sh
   dcmfx modify input.dcm --in-place --delete-tags 7FE00010
   ```

   Multiple data elements can be deleted by using a comma as a separator:

   ```sh
   dcmfx modify input.dcm --in-place --delete-tags 00100010,00100030
   ```

## Gleam CLI

The above examples assume the Rust version of the CLI tool is in use, however
most functionality is also supported by the Gleam version of the CLI, which can be
run with `gleam run` in `/src/gleam/dcmfx_cli`.

The Gleam CLI is primarily used when working on the Gleam implementation of
DCMfx. The Rust CLI is recommended for regular use as it is faster, has more
features, and is a single standalone binary.
