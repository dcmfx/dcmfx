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
    gleam test --target javascript --runtime bun
  fi

  cd ..
done

echo ""
echo "Done"
