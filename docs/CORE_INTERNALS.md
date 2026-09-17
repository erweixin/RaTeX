# Core implementation boundaries

This guide describes private implementation modules. Public Rust import paths,
AST and layout types, C ABI and DisplayList JSON remain unchanged.

## Same-machine refactor acceptance

The ignored `refactor_compatibility` integration test captures all three golden
corpora. Each record includes the complete AST, unrounded box dimensions,
DisplayList, an inline/color/tracking variant and the render/error result.
Successful cases also produce PNGs using the normal golden size and DPR.

Before modifying production code:

```bash
RATEX_COMPAT_OUTPUT="$PWD/target/refactor-compat/before" \
  cargo test --release -p ratex-render --test refactor_compatibility -- --ignored --nocapture
```

After the refactor, on the same machine with the same fonts and build features:

```bash
RATEX_COMPAT_OUTPUT="$PWD/target/refactor-compat/after" \
RATEX_COMPAT_BASELINE="$PWD/target/refactor-compat/before" \
  cargo test --release -p ratex-render --test refactor_compatibility -- --ignored --nocapture
```

Output directories must be new; the test refuses to overwrite a baseline. JSON
records are compared structurally, including array order. Identical PNG bytes
also guarantee identical dimensions and pixels with the same encoder. Known
unsupported formulas must preserve their error result, not disappear.

For authoritative visual scoring, use `tools/golden_compare/compare_golden.py`
and the workflow in `GOLDEN_BASELINE.md`. Capture directories have `errors.log`
files accepted by `build_render_manifest.py`; use the original corpus and DPR
to create each `render-manifest.json`. Compare both captures against the same
reference images, then pass the before report as `--baseline-report` with
`--max-case-regression 0`. Capture artifacts do not replace committed reference
images or `tests/golden/baseline.json`.

Pass `--ce` for mhchem even when specifying custom output directories. For
prooftree, pass `--prooftree-tolerant` and an explicit policy file containing
`{"cases": {}}`; the main suite's indexed exclusions do not apply to that corpus.
Use `--require-manifests` with the main references. Existing mhchem/prooftree
reference images may lack generation manifests: the scorer still checks image
index coverage, and both refactor captures must use those same references.
