# Example: Read DICOM Pixel Data

The following code reads a DICOM P10 file and then prints the size in bytes of
each frame of pixel data. The Rust version also writes each frame of pixel data
to a PNG file.

:::tabs key:code-example
== Gleam
<<< @/../../examples/dicom_read_pixel_data/gleam/src/dicom_read_pixel_data.gleam
== Rust
<<< @/../../examples/dicom_read_pixel_data/rust/src/main.rs
:::

[View on GitHub](https://github.com/dcmfx/dcmfx/tree/main/examples/dicom_read_pixel_data)
