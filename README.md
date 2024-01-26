<div align="center">
  <img src="https://emoji2svg.deno.dev/api/🩻" height="160px">
  <h1>DCMfx</h1>
  <p>
    <strong>
      Tools and libraries for working with DICOM, the international
      <br/>
      standard for medical images and related information
    </strong>
    <br />
  </p>

  [<img src="https://img.shields.io/github/v/release/dcmfx/dcmfx">](https://github.com/dcmfx/dcmfx/releases/latest)
  [<img src="https://img.shields.io/badge/semantic--release-angular-e10079?logo=semantic-release">](https://github.com/semantic-release/semantic-release)
  [<img src="https://github.com/dcmfx/dcmfx/actions/workflows/test.yml/badge.svg">](https://github.com/dcmfx/dcmfx/actions/workflows/test.yml)
  [<img src="https://img.shields.io/badge/License-AGPLv3-blue.svg">](https://www.gnu.org/licenses/agpl-3.0.en.html)
  [<img src="https://img.shields.io/badge/Gleam-1.5-FFAFF3">](https://gleam.run)
  [<img src="https://img.shields.io/badge/MSRV-1.80-CE422B">](https://www.rust-lang.org)
</div>

## DCMfx CLI 🔧

The DCMfx CLI tool exposes many capabilities of DCMfx for viewing, converting
and modifying DICOM and DICOM JSON files. Download the latest version
[here](https://github.com/dcmfx/dcmfx/releases/latest), or install with
[Homebrew](https://brew.sh) on macOS and Linux:

```sh
brew tap dcmfx/tap
brew install dcmfx
```

After installation, run `dcmfx` to see the available commands. The
[documentation](./docs/cli.md) contains more usage examples.

## DCMfx VS Code Extension 🛠️

The [DCMfx VS Code extension](https://github.com/dcmfx/dcmfx-vscode) views and
converts DICOM and DICOM JSON files in Visual Studio Code.

## DCMfx Playground 🛝

The [DCMfx Playground](https://github.com/dcmfx/dcmfx-playground) is a browser
app for viewing and converting DICOM and DICOM JSON files.

## DCMfx Libraries 📚

DCMfx is built from the following component libraries:

- **`dcmfx_core`**. DICOM data sets, data elements, value representations, value
  multiplicity, transfer syntaxes, and other DICOM concepts. Provides a registry
  of all data elements defined in Part 6 of the DICOM specification. as well as
  well-known private data elements.
- **`dcmfx_p10`**. Reads, writes, and modifies the DICOM Part 10 (P10) file
  format. Uses a streaming design suited for highly concurrent and
  memory-constrained environments.
- **`dcmfx_json`**. Converts between DICOM P10 and DICOM JSON.
- **`dcmfx_pixel_data`**. Extracts pixel data from a DICOM data set.
- **`dcmfx_anonymize`**. Provides functions for anonymizing a DICOM data set.
- **`dcmfx_character_set`**. Decodes DICOM string data to UTF-8, with full
  support for all DICOM character sets including ISO/IEC 2022 extensions.

The above libraries are not currently available as prebuilt packages or crates.

Future additions may include components for structured report data, pixel data
decoding, waveform data decoding, DICOMDIR indexes, and DIMSE networking.

## Languages 👨🏼‍💻

DCMfx is written in [Gleam](https://gleam.run), which allows it to be used from
Gleam, Elixir, Erlang, JavaScript, and TypeScript. It's the only DICOM library
that runs natively on the BEAM VM. The Gleam code is stored under
[`gleam/`](./gleam).

There is an official Rust port with an identical design and very similar API.
The Rust version is faster with lower memory usage and supports WASM build
targets. The Rust code is stored under [rust/](./rust).

## Validation ✅

DCMfx is thoroughly validated against other established DICOM implementations,
and its integration test suite has over 200 DICOM P10 files.

## License 📋

DCMfx is published under the GNU Affero General Public License Version 3
(AGPLv3). This license permits commercial use; however, any software that
incorporates DCMfx, either directly or indirectly, must also be released under
the AGPLv3 or a compatible license. This includes making the source code of the
combined work available under the same terms, and ensuring that users who
interact with the software over a network can access the source code.

Copyright © Dr Richard Viney, 2024.
