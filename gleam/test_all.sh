#/bin/sh
#
# Runs the tests for all dcmfx_* libraries in this directory,

set -e

for dir in dcmfx_*; do
  echo ""
  echo ""

  echo "Checking $dir ..."
  cd "$dir"

  gleam format --check .

  if [ "$dir" != "dcmfx_registry_codegen" ]; then
    gleam test --target erlang

    # The CLI doesn't support the JavaScript target
    if [ "$dir" != "dcmfx_cli" ]; then
      gleam test --target javascript
    fi
  fi

  cd ..
done
