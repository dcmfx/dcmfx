::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

# `dcmfx_pixel_data`

Extracts frames of pixel data from a DICOM data set.

At present this library only returns the raw bytes for each frame of pixel data.
Decoding and decompression of pixel data is not yet supported.

## Usage

### Gleam

```sh
gleam add dcmfx_core dcmfx_p10 dcmfx_pixel_data
```

```gleam
import dcmfx_core/data_set
import dcmfx_p10
import dcmfx_pixel_data
import gleam/bit_array
import gleam/list
import gleam/int

pub fn main() {
  // Read DICOM file
  let assert Ok(ds) = dcmfx_p10.read_file("input.dcm")

  // Read all frames of pixel data
  let assert Ok(#(_vr, frames)) = dcmfx_pixel_data.get_pixel_data(ds)

  // Print the size of each frame in bytes
  frames
  |> list.each(fn (frame) {
    let frame_size =
      frame
      |> list.reduce(0, fn(acc, bytes) { acc + bit_array.byte_size(bytes)})

    io.println("Found frame with size: " <> int.to_string(frame_size))
  })
}
```

### Rust

```sh
cargo add dcmfx
```

```rust
use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::pixel_data::*;

pub fn main() {
  // Read DICOM file
  let ds = DataSet::read_p10_file("input.dcm").unwrap();

  // Read all frames of pixel data
  let (_vr, frames) = ds.get_pixel_data().unwrap();

  // Print the size of each frame in bytes
  for frame in frames {
    let frame_size = frame.iter().reduce(0, |acc, bytes| acc + bytes.len());

    println!("Found frame with size: {}", frame_size);
  }
}
```
