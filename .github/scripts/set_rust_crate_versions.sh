#/bin/sh

set -e

# Update crate Cargo.toml files
find src/rust -name "Cargo.toml" -exec sed -i'' -E "s/^version = \".*\"$/version = \"$1\"/" {} +

# Update crate Cargo.lock files
cargo update --manifest-path src/rust/Cargo.toml -p dcmfx

# Update example app Cargo.lock files
for dir in examples/dicom_*/rust; do
  cargo update --manifest-path $dir/Cargo.toml
done
