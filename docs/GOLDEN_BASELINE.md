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
Ubuntu runner family. The full CI artifact report records:

- commit SHA and whether the local worktree was dirty;
- raw test-case SHA-256 and canonical suite hash;
- fixture and output manifest SHA-256 values;
- metric version (`ratex-ink-v2`);
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

The committed compact baseline is also consumed by the website, but CI does
not use its scores as the regression gate because browser/font rasterization
is OS-dependent. CI checks out the target commit into a temporary worktree and
regenerates its RaTeX output, KaTeX fixtures, and compact baseline on the same
Ubuntu runner as the proposed change. The per-case comparison therefore
measures the code change rather than differences between developer and runner
rendering environments.

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


## Centered negation (`\centernot`)

Run `./scripts/update_golden_centernot.sh` to render
`tests/golden/test_cases_centernot.txt`, regenerate its independent references,
and score every case with the authoritative Python comparator. The script
requires complete coverage and every case to pass the comparator's existing
0.30 per-case threshold. The main suite's 0.85 mean and 0.05 regression gates
remain unchanged. The script writes all diff images to
`tests/golden/diffs_centernot/` and the report to
`tests/golden/reports/centernot/report.json`.

KaTeX 0.16.45 does **not** implement `\centernot`. The dedicated generator's
`--centernot` flag installs a reference-only macro; it does not change the
main suite or replace RaTeX's native implementation. The reference follows
[centernot.sty v1.4, implementation lines 8–18](https://ctan.math.utah.edu/ctan/tex-archive/macros/latex/contrib/oberdiek/centernot.pdf):
place zero-advance `\not` at `(W - E) / 2`, where `W` is the entire argument's
width and `E` is the width of `=`, using the current math style. Keep the
baseline, argument advance, and outer relation class.

The independent KaTeX box construction is:

```latex
\mathrel{\mathclap{{\phantom{#1}}{\not}{\phantom{=}}}{#1}}
```

Inside `\mathclap`, the phantoms have combined width `W + E`, so the clap
begins at `-(W + E)/2`.
The first phantom advances by `W`, placing `\not` at `(W - E)/2`.
The final argument supplies its original advance and is painted after the
slash, matching the package's compositing order. Braced groups make each
phantom and the zero-width negation an ordinary atom inside the construction,
preventing accidental inter-atom relation glue during measurement. Phantom
contents remain invisible, and the argument is painted only once. The manifest records the
exact reference macro. This oracle validates the original package semantics
using KaTeX's independent layout; it is not a comparison against a native
KaTeX centernot implementation or a TeX rasterizer.

The suite covers all three issue #159 examples, narrow/plain/long/extensible
relations, compound arguments, scripts, math styles, color, font commands,
tall/empty arguments, and nesting. Rust geometry tests additionally distinguish
`\centernot` from left-aligned `\not` and check spacing and exact positioning.
