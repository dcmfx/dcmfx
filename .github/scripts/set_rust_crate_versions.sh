#/bin/sh

set -e

echo "Updating version in Cargo.toml files …"
find src/rust -name "Cargo.toml" -exec sed -i'' -E "s/^version = \".*\"$/version = \"$1\"/" {} +

echo "Updating main Cargo.lock …"
cargo update --manifest-path src/rust/Cargo.toml -p dcmfx

echo "Updating example app Cargo.lock files …"
for dir in examples/dicom_*/rust; do
  
  cargo update --manifest-path $dir/Cargo.toml
done

echo "Updating fuzzer Cargo.lock …"
cargo update --manifest-path src/rust/dcmfx_fuzz/Cargo.toml
