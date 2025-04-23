#/bin/sh

set -e

echo "Updating version in gleam.toml files …"
find src/gleam -name "gleam.toml" -exec sed -i'' -E "s/^version = \".*\"$/version = \"$1\"/" {} +

echo "Updating main manifest.toml files …"
for dir in src/gleam/dcmfx_*/; do
  cd $dir
  gleam deps update dcmfx_anonymize dcmfx_character_set dcmfx_core dcmfx_json dcmfx_p10 dcmfx_pixel_data
  cd ../../..
done

# Update the hardcoded version in uids.toml. In future perhaps Gleam will have
# the ability to insert the package version at build time, which would avoid
# needing to hardcode this.
echo "Updating version in uids.gleam …"
sed -i'' -E "s/\"DCMfx \" <> \".*\"$/\"DCMfx \" <> \"$1\"/" src/gleam/dcmfx_p10/src/dcmfx_p10/uids.gleam

echo "Updating example app manifest.toml files …"
for dir in examples/dicom_*/gleam; do
  cd $dir
  gleam deps update dcmfx_anonymize dcmfx_character_set dcmfx_core dcmfx_json dcmfx_p10 dcmfx_pixel_data
  cd ../../..
done
