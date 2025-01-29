#!/bin/bash

rm -rf build

# Find all text files recursively and process them
find . -type f -name "*.rs" | while read -r file; do
  awk 'length($0) > 80 { print FILENAME ": " $0 }' "$file"
done
