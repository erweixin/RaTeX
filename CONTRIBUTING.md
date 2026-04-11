# Contributing to RaTeX

Thanks for helping improve RaTeX. Keep changes focused and consistent with surrounding code.

## Prerequisites

- **Rust**: stable toolchain ([rustup](https://rustup.rs)); see README for minimum version.
- **Web / WASM builds**: [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) when working under `platforms/web`.

## Build, lint, test

From the repository root:

```bash
cargo build --workspace
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

CI runs the same checks (`.github/workflows/ci.yml`).

## Golden (visual) tests

Reference PNGs live under `tests/golden/fixtures/`. Regenerate RaTeX outputs with:

```bash
./scripts/update_golden_output.sh
```

Comparison helpers: `tools/golden_compare/`, and `crates/ratex-render/tests/golden_test.rs`. Some cases score lower than others due to font subpixel rendering, anti-aliasing, or layout edge differences versus KaTeX reference PNGs — that does not always indicate a visible bug.

**mhchem (`\ce` / `\pu`) golden**: reference PNGs in `tests/golden/fixtures_ce/` (KaTeX + mhchem, via Puppeteer):

```bash
cd tools/golden_compare && npm install
node generate_reference.mjs ../../tests/golden/test_case_ce.txt ../../tests/golden/fixtures_ce --mhchem
```

Ink score for that suite:

```bash
cargo test -p ratex-render golden_mhchem_pass_rate -- --nocapture
```

RaTeX renders for inspection: `./scripts/update_golden_output.sh` (writes `tests/golden/output_ce/`). Compare with KaTeX refs using `python3 tools/golden_compare/compare_golden.py --ce` (same ink metric as the main golden script).

## Regenerating font data (advanced)

KaTeX-derived metrics/symbols in `crates/ratex-font/src/data/` are generated from scripts in `tools/` (`convert_metrics.py`, `convert_symbols.py`). Only rerun when intentionally updating KaTeX baseline data.

## Pull requests

- One logical change per PR when possible.
- If behavior or public API changes, update the relevant README or `docs/` note.
- For release/version bumps, follow `RELEASING.md`.

## Project layout

See [`docs/PROJECT_STRUCTURE.md`](docs/PROJECT_STRUCTURE.md).
