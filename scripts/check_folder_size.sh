#!/usr/bin/env bash
set -euo pipefail

MAX_MB=200
BAD=0

# Use a narrow scope if desired (project folders) to avoid massive overhead.
# This checks all directories under the repo root (including nested). Change to /path/to/folders for targeted checks.
find . -type d -print0 | while IFS= read -r -d '' dir; do
  size_mb=$(du -sm "$dir" 2>/dev/null | awk '{print $1}')
  if [[ -n "$size_mb" && "$size_mb" -gt "$MAX_MB" ]]; then
    echo "ERROR: $dir is ${size_mb}MB (> ${MAX_MB}MB)"
    BAD=1
  fi
done

if [[ "$BAD" -ne 0 ]]; then
  echo "\nFolder size policy violation detected. Reduce folder size below ${MAX_MB}MB."
  exit 1
fi

echo "Folder size policy check passed (all dirs <= ${MAX_MB}MB)."