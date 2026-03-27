#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FONT_DIR="$ROOT/tools/lexer_compare/node_modules/katex/dist/fonts"
OUTPUT_DIR="$ROOT/tests/golden/output"
OUTPUT_CE_DIR="$ROOT/tests/golden/output_ce"
OUTPUT_SVG_DIR="$ROOT/tests/golden/output_svg"
OUTPUT_SVG_CE_DIR="$ROOT/tests/golden/output_svg_ce"
TEST_CASES="$ROOT/tests/golden/test_cases.txt"
TEST_CASE_CE="$ROOT/tests/golden/test_case_ce.txt"
TMP_ERR="$(mktemp)"
TMP_ERR_CE="$(mktemp)"
TMP_ERR_SVG="$(mktemp)"
TMP_ERR_SVG_CE="$(mktemp)"
trap 'rm -f "$TMP_ERR" "$TMP_ERR_CE" "$TMP_ERR_SVG" "$TMP_ERR_SVG_CE"' EXIT

echo "Building ratex-render (release)..."
cargo build --release -p ratex-render

echo "Building ratex-svg render-svg (release, cli+standalone)..."
cargo build --release -p ratex-svg --features cli,standalone --bin render-svg

mkdir -p "$OUTPUT_DIR" "$OUTPUT_SVG_DIR"

echo "Clearing old PNG/SVG output..."
rm -f "$OUTPUT_DIR"/*.png
rm -f "$OUTPUT_SVG_DIR"/*.svg

echo "Rendering formulas (PNG)..."
cargo run --release -p ratex-render --bin render -- \
  --font-dir "$FONT_DIR" \
  --output-dir "$OUTPUT_DIR" \
  < "$TEST_CASES" 2>"$TMP_ERR"

echo "Rendering formulas (SVG, path glyphs)..."
(cd "$ROOT" && cargo run --release -p ratex-svg --features cli,standalone --bin render-svg -- \
  --font-dir "$FONT_DIR" \
  --output-dir "$OUTPUT_SVG_DIR" \
  < "$TEST_CASES") 2>"$TMP_ERR_SVG"

if [[ -s "$TMP_ERR" ]]; then
  failed_count=$(grep -c '^ERR' "$TMP_ERR" 2>/dev/null || true)
  echo ""
  echo "PNG failed: $failed_count case(s)"
  grep '^ERR' "$TMP_ERR" || true
fi

if [[ -s "$TMP_ERR_SVG" ]]; then
  failed_svg=$(grep -c '^ERR' "$TMP_ERR_SVG" 2>/dev/null || true)
  echo ""
  echo "SVG failed: $failed_svg case(s)"
  grep '^ERR' "$TMP_ERR_SVG" || true
fi

# ── mhchem / \\ce / \\pu suite ──────────────────────────
if [[ -f "$TEST_CASE_CE" ]]; then
  echo ""
  echo "Rendering mhchem suite (test_case_ce.txt) → output_ce/ + output_svg_ce/..."
  rm -f "$OUTPUT_CE_DIR"/*.png
  rm -f "$OUTPUT_SVG_CE_DIR"/*.svg
  mkdir -p "$OUTPUT_CE_DIR" "$OUTPUT_SVG_CE_DIR"
  : >"$TMP_ERR_CE"
  : >"$TMP_ERR_SVG_CE"
  # Match KaTeX reference pixel density (Puppeteer deviceScaleFactor 2) for ink comparison.
  # If fixtures_ce were regenerated with DPR 1 (see generate_reference.mjs), use --dpr 1 here.
  cargo run --release -p ratex-render --bin render -- \
    --font-dir "$FONT_DIR" \
    --output-dir "$OUTPUT_CE_DIR" \
    --dpr 2 \
    < "$TEST_CASE_CE" 2>"$TMP_ERR_CE"
  (cd "$ROOT" && cargo run --release -p ratex-svg --features cli,standalone --bin render-svg -- \
    --font-dir "$FONT_DIR" \
    --output-dir "$OUTPUT_SVG_CE_DIR" \
    --dpr 2 \
    < "$TEST_CASE_CE") 2>"$TMP_ERR_SVG_CE"
  if [[ -s "$TMP_ERR_CE" ]]; then
    failed_ce=$(grep -c '^ERR' "$TMP_ERR_CE" 2>/dev/null || true)
    echo "mhchem PNG render errors: $failed_ce"
    grep '^ERR' "$TMP_ERR_CE" || true
  fi
  if [[ -s "$TMP_ERR_SVG_CE" ]]; then
    failed_svg_ce=$(grep -c '^ERR' "$TMP_ERR_SVG_CE" 2>/dev/null || true)
    echo "mhchem SVG render errors: $failed_svg_ce"
    grep '^ERR' "$TMP_ERR_SVG_CE" || true
  fi
fi

echo "Done."
