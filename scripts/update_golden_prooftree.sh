#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MARKER="KaTeX_Main-Regular.ttf"
if [[ -f "$ROOT/fonts/$MARKER" ]]; then
  FONT_DIR="$ROOT/fonts"
elif [[ -f "$ROOT/crates/ratex-katex-fonts/fonts/$MARKER" ]]; then
  FONT_DIR="$ROOT/crates/ratex-katex-fonts/fonts"
elif [[ -f "$ROOT/tools/lexer_compare/node_modules/katex/dist/fonts/$MARKER" ]]; then
  FONT_DIR="$ROOT/tools/lexer_compare/node_modules/katex/dist/fonts"
else
  FONT_DIR="$ROOT/fonts"
  echo "WARNING: $MARKER not found; PNG/SVG may fail." >&2
fi

OUTPUT_DIR="$ROOT/tests/golden/output_prooftree"
OUTPUT_SVG_DIR="$ROOT/tests/golden/output_svg_prooftree"
TEST_CASES="$ROOT/tests/golden/test_cases_prooftree.txt"
TMP_ERR="$(mktemp)"
TMP_ERR_SVG="$(mktemp)"
trap 'rm -f "$TMP_ERR" "$TMP_ERR_SVG"' EXIT

export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=128
export CARGO_PROFILE_RELEASE_INCREMENTAL=true

echo "Building ratex-render (release)..."
cargo build --release -p ratex-render

echo "Building ratex-svg render-svg (release, cli+standalone)..."
cargo build --release -p ratex-svg --features cli,standalone --bin render-svg

mkdir -p "$OUTPUT_DIR" "$OUTPUT_SVG_DIR"
rm -f "$OUTPUT_DIR"/*.png "$OUTPUT_SVG_DIR"/*.svg

echo "Rendering prooftree formulas (PNG)..."
# Render errors are informational here (see update_golden_output.sh). `|| true` keeps
# `set -e` from aborting on the binary's non-zero exit; failures are reported below.
cargo run --release -p ratex-render --bin render -- \
  --font-dir "$FONT_DIR" \
  --font-size 36 \
  --output-dir "$OUTPUT_DIR" \
  < "$TEST_CASES" 2>"$TMP_ERR" || true

echo "Rendering prooftree formulas (SVG, path glyphs)..."
(cd "$ROOT" && cargo run --release -p ratex-svg --features cli,standalone --bin render-svg -- \
  --font-dir "$FONT_DIR" \
  --font-size 36 \
  --output-dir "$OUTPUT_SVG_DIR" \
  < "$TEST_CASES") 2>"$TMP_ERR_SVG" || true

if [[ -s "$TMP_ERR" ]]; then
  failed=$(grep -c '^ERR' "$TMP_ERR" 2>/dev/null || true)
  echo "PNG render errors: $failed"
  grep '^ERR' "$TMP_ERR" || true
fi
if [[ -s "$TMP_ERR_SVG" ]]; then
  failed_svg=$(grep -c '^ERR' "$TMP_ERR_SVG" 2>/dev/null || true)
  echo "SVG render errors: $failed_svg"
  grep '^ERR' "$TMP_ERR_SVG" || true
fi

echo "Done."
