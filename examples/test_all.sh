# Runs all examples in this directory to check they work.

set -euo pipefail

cargo fmt --check
cargo clippy -- --deny warnings

for dir in dicom_*; do
  echo ""
  echo "Testing $dir …"

  if [ -d "$dir/gleam" ]; then
    cd $dir/gleam
    gleam format --check .
    gleam run --target erlang
    gleam run --target javascript --runtime node
    gleam run --target javascript --runtime deno
    gleam run --target javascript --runtime bun

    cd ../..
  fi

  cargo run --locked -p $dir
done

echo ""
echo "Done"
