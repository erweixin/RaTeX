#!/bin/bash
# Install Git hooks (e.g. pre-commit runs cargo fmt and cargo clippy).
# Run from repo root: ./scripts/install-hooks.sh

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if [[ ! -d .githooks ]]; then
  echo "No .githooks directory found." >&2
  exit 1
fi

# Option A: use core.hooksPath so all hooks live in .githooks (tracked by git)
git config core.hooksPath .githooks
echo "Git hooks path set to .githooks (pre-commit will run cargo fmt and cargo clippy)."
chmod +x .githooks/pre-commit 2>/dev/null || true
