# DCMfx CLI

The DCMfx CLI tool is available for download for all major OS's on the
[Releases](https://github.com/dcmfx/dcmfx/releases) page. It can also be
installed using [Homebrew](https://brew.sh) on macOS and Linux:

```sh
brew tap dcmfx/tap
brew install dcmfx
```

## Examples

1. Print a DICOM P10 file's data set to stdout:

   ```
   dcmfx print input.dcm
   ```

2. Convert a DICOM P10 file to a DICOM JSON file:

   ```
   dcmfx to-json input.dcm output.json
   ```

3. Convert a DICOM JSON file to a DICOM P10 file:

   ```
   dcmfx to-dcm input.json output.dcm
   ```

4. Extract pixel data from a DICOM P10 file to image files:

   ```
   dcmfx extract-pixel-data input.dcm
   ```

5. Modify a DICOM P10 file. This will convert the specific character set to
   UTF-8, as well as change sequences and items to undefined length:

   ```
   dcmfx modify input.dcm output.dcm
   ```

6. Modify a DICOM P10 file to use the 'Deflated Explicit VR Little Endian'
   transfer syntax with maximum compression:

   ```
   dcmfx modify input.dcm output.dcm \
     --transfer-syntax=deflated-explicit-vr-little-endian \
     --zlib-compression-level=9
   ```

   The `modify` command can only convert between the following transfer
   syntaxes:

   - Implicit VR Little Endian
   - Explicit VR Little Endian
   - Deflated Explicit VR Little Endian
   - Explicit VR Big Endian

   Conversion between other transfer syntaxes is not supported.

7. Anonymize a DICOM P10 file to have all identifying data elements removed:

   ```
   dcmfx modify input.dcm output.dcm --anonymize
   ```

   Note that this does not remove any identifying information baked into pixel
   data or other binary data elements.

8. Remove the top-level '(7FE0,0000) Pixel Data' data element from a DICOM P10
   file:

   ```
   dcmfx modify input.dcm output.dcm --delete-tags=7FE00010
   ```

   Multiple data elements can be deleted by using a comma as a separator:

   ```
   dcmfx modify input.dcm output.dcm --delete-tags=00100010,00100030
   ```

> [!INFORMATION]
> The above examples assume the Rust version of the CLI tool is in use. All
> commmands are also supported by the Gleam version of the CLI, which can be run
> with `gleam run` in the `/gleam/dcmfx_cli` directory.
