#/bin/sh
#
# Runs all examples in this directory to check they work.

set -e

for dir in */; do
  echo ""
  echo "Testing $dir …"

  cd "$dir"/gleam
  gleam format --check .
  gleam run --target erlang
  gleam run --target javascript --runtime node
  gleam run --target javascript --runtime deno

  cd ../rust
  cargo fmt --check
  cargo clippy -- --deny warnings
  cargo run

  cd ../..
done

echo ""
echo "Done"
