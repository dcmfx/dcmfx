#/bin/sh

set -e

# Update crate Cargo.toml files
echo "Updating version in Cargo.toml files …"
find src/rust -name "Cargo.toml" -exec sed -i'' -E "s/^version = \".*\"$/version = \"$1\"/" {} +

# Update crate Cargo.lock files
echo "Updating main Cargo.lock …"
cargo update --manifest-path src/rust/Cargo.toml -p dcmfx

# Update example app Cargo.lock files
echo "Updating example app Cargo.lock files …"
for dir in examples/dicom_*/rust; do
  
  cargo update --manifest-path $dir/Cargo.toml
done

# Update
echo "Updating fuzzer Cargo.lock …"
cargo update --manifest-path src/rust/dcmfx_fuzz/Cargo.toml
