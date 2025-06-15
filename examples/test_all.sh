#!/bin/sh
#
# Runs all examples in this directory to check they work.

set -e

cargo fmt --check

for dir in dicom_*; do
  echo ""
  echo "Testing $dir …"

  cd $dir/gleam
  gleam format --check .
  gleam run --target erlang
  gleam run --target javascript --runtime node
  gleam run --target javascript --runtime deno
  gleam run --target javascript --runtime bun

  cd ../..

  cargo run --locked -p $dir
done

echo ""
echo "Done"
