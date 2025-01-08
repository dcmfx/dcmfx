# Design

DCMfx's design is centered on streaming of DICOM data, meaning all operations
are performed in a streaming fashion wherever possible, enabling fast execution
with extremely low memory usage regardless of DICOM or data set size.

Loading DICOM data sets completely into memory is also supported and can be
simpler for tasks where resource constraints are not a concern.

## Languages

DCMfx is dual-implemented in two languages: [Gleam](https://gleam.run) and
[Rust](https://rust-lang.org). See [here](./libraries/overview#languages) for
more details.
