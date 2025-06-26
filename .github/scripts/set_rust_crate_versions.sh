#!/bin/sh

set -e

echo "Updating version in Cargo.toml files …"
find src/rust -name "Cargo.toml" -exec sed -i'' -E "s/^version = \".*\"$/version = \"$1\"/" {} +

echo "Updating main Cargo.lock …"
cargo update --manifest-path src/rust/Cargo.toml -p dcmfx

echo "Updating examples Cargo.lock file …"
cargo update --manifest-path examples/Cargo.toml

echo "Updating fuzzer Cargo.lock …"
cargo update --manifest-path src/rust/dcmfx_fuzz/Cargo.toml

echo "Updating WASM test Cargo.lock …"
cargo update --manifest-path src/rust/dcmfx_wasm_test/Cargo.toml
