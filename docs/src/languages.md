# Languages

DCMfx is dual-implemented in two languages: [Gleam](https://gleam.run) and
[Rust](https://rust-lang.org). The two implementations have identical designs
and very similar APIs.

## Gleam

The Gleam implementation allows DCMfx to be used directly from Gleam, Elixir,
Erlang, JavaScript, and TypeScript.

It's also the only DICOM library that runs natively on the BEAM VM.

## Rust

The Rust implementation allows DCMfx to be used from Rust, WASM, and is faster
with lower memory usage.

The Rust implementation is used for the DCMfx CLI tool.
