#!/usr/bin/env bash
# Fail if an iOS static library's unwind metadata references Rust's personality
# routine. Generic compiler_builtins unwind entries without a personality are
# valid and do not consume a compact-unwind personality slot.

set -euo pipefail

if [[ "$#" -lt 1 ]]; then
  echo "Usage: $0 <static-library> [static-library ...]" >&2
  exit 2
fi

if ! command -v otool >/dev/null 2>&1; then
  echo "::error::otool is required to inspect Apple static libraries." >&2
  exit 1
fi

for library in "$@"; do
  if [[ ! -f "$library" ]]; then
    echo "::error::Static library not found: $library" >&2
    exit 1
  fi

  for arch in $(lipo -archs "$library"); do
    personality_relocations="$(
      otool -arch "$arch" -rv "$library" |
        awk '$NF == "_rust_eh_personality" { count++ } END { print count + 0 }'
    )"

    if [[ "$personality_relocations" -ne 0 ]]; then
      echo "::error::_rust_eh_personality is referenced by unwind metadata in $library ($arch): $personality_relocations relocation(s)" >&2
      exit 1
    fi

    echo "No _rust_eh_personality unwind relocations: $library ($arch)"
  done
done

echo "iOS static library personality check passed."
