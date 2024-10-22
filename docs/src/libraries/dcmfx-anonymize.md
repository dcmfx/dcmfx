::: tip UNRELEASED
This library is not yet published as a package or crate.
:::

# `dcmfx_anonymize`

This library provides anonymization of DCMfx data sets by removing data elements
that identify the patient, or potentially contribute to identification of the
patient.

## Usage

### Gleam

```sh
gleam add dcmfx_anonymize dcmfx_core dcmfx_p10
```

```gleam
import dcmfx_anonymize
import dcmfx_core/data_set
import dcmfx_p10
import gleam/option.{None}

pub fn main() {
  let assert Ok(ds) = dcmfx_p10.read_file("input.dcm")

  let ds = dcmfx_anonymize.anonymize_data_set(ds)

  data_set.print(ds, None)
}
```

### Rust

```sh
cargo add dcmfx
```

```rust
use dcmfx::core::*;
use dcmfx::p10::*;
use dcmfx::anonymize::*;

pub fn main() {
  let mut ds = DataSet::read_p10_file("input.dcm").unwrap();

  ds.anonymize();

  ds.print(None);
}
```
