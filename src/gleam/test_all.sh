#/bin/sh
#
# Runs the tests for all dcmfx_* libraries in this directory,

set -e

for dir in dcmfx_*; do
  echo ""
  echo "Testing $dir …"

  cd "$dir"
  gleam format --check

  if [ "$dir" != "dcmfx_dictionary_codegen" ]; then
    gleam test --target erlang
    gleam test --target javascript --runtime node
    gleam test --target javascript --runtime deno

    # The Bun JavaScript runtime will be supported once the crash described in
    # https://github.com/oven-sh/bun/issues/13233 is fixed.
  fi

  cd ..
done

echo ""
echo "Done"
