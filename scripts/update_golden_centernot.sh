#!/bin/bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
CASES=tests/golden/test_cases_centernot.txt
OUTPUT=tests/golden/output_centernot
FIXTURES=tests/golden/fixtures_centernot
REPORTS=tests/golden/reports/centernot
mkdir -p "$OUTPUT" "$REPORTS"
TMP_ERR="$(mktemp)"
trap 'rm -f "$TMP_ERR"' EXIT
cargo build --release -p ratex-render --bin render
rm -f "$OUTPUT"/*.png "$OUTPUT/render-manifest.json"
cargo run --release -p ratex-render --bin render -- \
  --font-dir fonts --output-dir "$OUTPUT" < "$CASES" 2>"$TMP_ERR"
python3 tools/golden_compare/build_render_manifest.py \
  --test-cases "$CASES" --output "$OUTPUT" --error-log "$TMP_ERR" \
  --json-out "$OUTPUT/render-manifest.json" --dpr 1
node tools/golden_compare/generate_reference.mjs "$CASES" "$FIXTURES" --centernot
python3 tools/golden_compare/compare_golden.py \
  --suite centernot --test-cases "$CASES" --fixtures "$FIXTURES" --output "$OUTPUT" \
  --policy tests/golden/policy_centernot.json \
  --require-manifests --fail-on-missing --min-coverage 1.0 --threshold 0.30 \
  --diff-dir tests/golden/diffs_centernot --diff-from 1 \
  --json-out "$REPORTS/report.json"
# The comparator's threshold labels per-case passes; enforce them for this suite.
python3 - "$REPORTS/report.json" <<'PY_CHECK'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    report = json.load(handle)
summary = report["summary"]
if summary["passed_count"] != summary["eligible_count"]:
    raise SystemExit("centernot: one or more cases failed the standard diff threshold")
print(f"centernot: {summary['passed_count']}/{summary['eligible_count']} cases passed")
PY_CHECK
