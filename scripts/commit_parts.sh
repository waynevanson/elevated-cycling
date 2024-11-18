#!/bin/bash

# Check if any arguments are provided
if [ "$#" -lt 1 ]; then
  echo "Usage: $0 file1 [file2 ...]"
  exit 1
fi

# Iterate over the list of files
for file in "$@"; do
  if [ -f "$file" ]; then
    git add "$file"
    git commit -m "Add or update file: $file"
    echo "Committed: $file"
  else
    echo "File not found: $file"
  fi
done

echo "All files processed."
