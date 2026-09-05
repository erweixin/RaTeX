# Contributing to RaTeX

Thanks for helping improve RaTeX. Keep changes focused and consistent with surrounding code.

When restructuring core logic, follow the module boundaries and compatibility
capture workflow in [`docs/CORE_INTERNALS.md`](docs/CORE_INTERNALS.md).

## Prerequisites

- **Rust**: stable toolchain ([rustup](https://rustup.rs)); see README for minimum version.
- **Web / WASM builds**: [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) when working under `platforms/web`.
- **GTK checks**: full workspace checks include `crates/ratex-gtk4`, which needs GTK4/graphene development packages visible to `pkg-config`.

## Build, lint, test

From the repository root:

```bash
cargo build --workspace
cargo fmt --all -- --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

CI runs the same checks (`.github/workflows/ci.yml`) after installing GTK dependencies. The local pre-commit hook skips only `ratex-gtk4` when GTK4/graphene `pkg-config` files are missing, while still linting the rest of the workspace.

## Golden (visual) tests

Reference PNGs live under `tests/golden/fixtures/`. Regenerate RaTeX outputs with:

```bash
./scripts/update_golden_output.sh
```

The Python comparator is the only authoritative score source and CI gate. The
Rust test in `crates/ratex-render/tests/golden_test.rs` is a fast smoke test; it
must not be used to publish a second “official” mean. See
[`docs/GOLDEN_BASELINE.md`](docs/GOLDEN_BASELINE.md) for report fields,
integrity rules, exclusions, and baseline updates.

Generate the compact versioned baseline with:

```bash
cd tools/golden_compare && npm ci
cd ../..
python3 -m pip install -r tools/golden_compare/requirements.txt
./scripts/update_golden_baseline.sh
```

Only `tests/golden/baseline.json` is committed. Generated RaTeX PNG/SVG output,
manifests, CSV, and the full diagnostic report remain local or are uploaded as
CI artifacts.

**KaTeX syntax not supported or not equivalent (command-level):** see [README.md](README.md) and [README.zh-CN.md](README.zh-CN.md) (sections *KaTeX differences (commands & DOM)* / *与 KaTeX 的差异（命令 / DOM）*).

**mhchem (`\ce` / `\pu`) golden**: reference PNGs in `tests/golden/fixtures_ce/` (KaTeX + mhchem, via Puppeteer):

```bash
cd tools/golden_compare && npm install
node generate_reference.mjs ../../tests/golden/test_case_ce.txt ../../tests/golden/fixtures_ce --mhchem
```

Ink score for that suite:

```bash
cargo test -p ratex-render golden_mhchem_smoke -- --nocapture
```

## Stack safety

Parser-driven rendering uses a shared depth budget for input-controlled
recursive structures. Before adding or changing recursive syntax, read
[`docs/STACK_SAFETY.md`](docs/STACK_SAFETY.md) and add the boundary and
small-stack regressions described there.

RaTeX renders for inspection: `./scripts/update_golden_output.sh` (writes `tests/golden/output_ce/`). Compare with KaTeX refs using `python3 tools/golden_compare/compare_golden.py --ce` (same ink metric as the main golden script).

**bussproofs / `prooftree` golden**: test cases live in `tests/golden/test_cases_prooftree.txt`. KaTeX does not support `prooftree`, so reference PNGs are generated with MathJax and its bussproofs extension:

```bash
cd tools/golden_compare && npm install
node generate_reference_prooftree.mjs ../../tests/golden/test_cases_prooftree.txt ../../tests/golden/fixtures_prooftree
```

RaTeX renders for inspection:

```bash
./scripts/update_golden_prooftree.sh
```

This writes PNG output to `tests/golden/output_prooftree/` and standalone SVG output to `tests/golden/output_svg_prooftree/`. Compare against the MathJax references with:

```bash
python3 tools/golden_compare/compare_golden.py \
  --fixtures tests/golden/fixtures_prooftree \
  --output tests/golden/output_prooftree
```

## Regenerating font data (advanced)

KaTeX-derived metrics/symbols in `crates/ratex-font/src/data/` are generated from scripts in `tools/` (`convert_metrics.py`, `convert_symbols.py`). Only rerun when intentionally updating KaTeX baseline data.

## Pull requests

- One logical change per PR when possible.
- If behavior or public API changes, update the relevant README or `docs/` note.
- For release/version bumps, follow `RELEASING.md`.

## Project layout

See [`docs/PROJECT_STRUCTURE.md`](docs/PROJECT_STRUCTURE.md).
