# Runs fuzz testing on DCMfx using AFL.

set -e

# Install afl.rs
cargo install cargo-afl@0.15.17

# Remove all data from previous runs
rm -rf inputs outputs

# Copy test DICOM files to the inputs/ directory to be used for fuzzing
mkdir inputs
find ../../../test -type f -name '*.dcm' -exec cp {} inputs \;

# Build instrumented binary
cargo afl build --release

# Run the fuzzing. This will run indefinitely until it is terminated.
cargo afl fuzz -c 0 -i inputs -o outputs ./target/release/dcmfx_fuzz
