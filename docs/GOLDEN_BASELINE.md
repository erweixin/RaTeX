# Authoritative golden baseline

`tools/golden_compare/compare_golden.py` is the only authoritative scorer for
RaTeX visual-golden results. The Rust golden tests are quick smoke checks and
do not define a published mean or CI quality threshold.

## Fixed reference environment

The reference renderer is pinned to KaTeX 0.16.45. Do not upgrade KaTeX while
working toward a higher RaTeX score; a reference upgrade is a separate,
reviewed baseline change.

`tools/golden_compare/package-lock.json` pins KaTeX and Puppeteer exactly.
`.github/workflows/golden.yml` additionally fixes Node, Python, Rust, and the
Ubuntu runner family.

### System fonts are part of the pinned reference environment

The KaTeX reference is rendered in a headless Chromium and RaTeX renders `\text`
fallback (CJK, emoji, …) from system fonts, so the **installed system font set
must be pinned too** — it is a scoring input, not incidental.

- `golden.yml` installs `fonts-noto-cjk` and `fonts-noto-color-emoji` before
  rendering. RaTeX discovers CJK at the exact path `fonts-noto-cjk` installs
  (`/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc`) and emoji via the
  `Noto Color Emoji` family; without them CI renders CJK/emoji text as tofu and
  every such case shows up as a spurious golden regression.
- The committed `baseline.json` is authoritative only for the environment that
  produced it. A baseline generated on a machine with a different font set or a
  different OS will not reproduce in CI (scores drift in both directions).
  **Regenerate the baseline in CI** (download the `golden-report` artifact's
  `baseline.json`) or in an environment identical to the runner — do not commit
  a baseline produced on a machine whose fonts differ from the runner's.

The full CI artifact report records:

- commit SHA and whether the local worktree was dirty;
- raw test-case SHA-256 and canonical suite hash;
- fixture and output manifest SHA-256 values;
- metric version (`ratex-ink-v1`);
- actual KaTeX, Puppeteer, and Chromium versions/revisions;
- Rust, Cargo, Node, Python, Pillow, and NumPy versions;
- OS image metadata;
- reference and output DPR;
- SHA-256 values for every KaTeX/RaTeX font file used.

## Generated indexed manifests

Formula order defines the continuous case range `0001..NNNN`. Reference and
RaTeX generation each write a temporary manifest with one record per formula:

Blank lines and lines whose first non-whitespace character is `#` or `%` are
comments and do not consume an index. Every corpus reader follows this rule.

- `tests/golden/fixtures/reference-manifest.json`
- `tests/golden/output/render-manifest.json`

These manifests and all RaTeX-rendered output are ignored by Git. A failed
render still occupies an explicit indexed slot even when it has no PNG. The
report enforces:

```text
formula_count == fixture_count == output_count == report_case_count
```

It also reports the actual `fixture_png_count` and `output_png_count`.
Unexpected, non-canonical, duplicate, or out-of-range PNG names are integrity
errors.

Every report case has one of these statuses:

- `scored`
- `missing_fixture`
- `missing_output`
- `parse_error`
- `layout_error`
- `render_error`
- `unsupported`

Known exclusions must be listed in `tests/golden/policy.json`, including the
exact formula and a reason. Policy entries are checked for index/formula drift.
Unscored cases contribute zero to `coverage_adjusted_mean`. `rendered_mean`
only averages scored cases. `coverage` is scored cases divided by eligible
(non-policy) cases; `raw_coverage` includes policy exclusions in the
denominator.

## Generate and gate

Install the locked dependencies once:

```bash
cd tools/golden_compare
npm ci
cd ../..
python3 -m pip install -r tools/golden_compare/requirements.txt
```

Regenerate the committed baseline:

```bash
./scripts/update_golden_baseline.sh
```

Or invoke the comparator directly:

```bash
python3 tools/golden_compare/compare_golden.py \
  --baseline-out tests/golden/baseline.json \
  --require-manifests \
  --fail-on-missing \
  --min-coverage 1.0
```

The committed `baseline.json` is deliberately minified. It stores only the
metric, suite name/hash, and one rounded score (or `null`) per formula. Formula
text remains in `test_cases.txt`; it is not duplicated. Full JSON diagnostics
are uploaded by CI as an artifact, while CSV, generated manifests, PNG output,
and SVG output are not versioned.

Useful gates:

```text
--json-out PATH
--csv-out PATH
--baseline-out PATH
--fail-on-missing
--min-coverage 0..1
--min-mean 0..1
--baseline-report PATH
--baseline-formulas PATH
--max-case-regression 0..1
--require-manifests
```

`--min-mean` gates `coverage_adjusted_mean`. With a baseline report,
`--max-case-regression` treats an unscored current case as score zero and
fails when a matched formula drops by more than the allowed amount. Formulas
are matched by exact source text and duplicate occurrence order, so additions,
removals, and reordering are reported without preventing comparisons for
formulas present in both suites. Metric-version mismatches remain integrity
errors, as do attempts to compare different named suites. If two reports claim
the same suite hash but contain different cases, the report is treated as
inconsistent.

## Website source

`website/src/pages/demo/support-table.astro` combines
`tests/golden/test_cases.txt` with `tests/golden/baseline.json`. Do not add a
second embedded formula list or score map.
