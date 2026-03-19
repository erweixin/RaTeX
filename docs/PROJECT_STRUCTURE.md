# RaTeX Project Structure

Current layout as of the codebase. RA (Rust) + TeX.

---

## Root Layout

```
RaTeX/
├── Cargo.toml                    # Workspace root
├── README.md
├── CONTRIBUTING.md               # Build, test, golden workflow, PR notes
├── SECURITY.md                   # How to report vulnerabilities
├── LICENSE                       # MIT
├── .gitignore
├── .github/
│   └── workflows/
│       ├── ci.yml                # Build + Clippy + Test
│       ├── pages.yml             # GitHub Pages (demo)
│       └── release-*.yml         # crates.io, npm, pub.dev, iOS/Android/RN
│
├── crates/                       # Rust crates
│   ├── ratex-types/              # Shared types (DisplayList, Color, etc.)
│   ├── ratex-font/               # Font metrics + symbol tables (KaTeX-compatible)
│   ├── ratex-lexer/               # LaTeX → token stream
│   ├── ratex-parser/             # Token stream → ParseNode AST
│   ├── ratex-layout/             # AST → LayoutBox → DisplayList
│   ├── ratex-ffi/                # C ABI: LaTeX → DisplayList JSON (+ Android JNI)
│   ├── ratex-render/             # DisplayList → PNG (tiny-skia, server-side)
│   └── ratex-wasm/               # WASM: LaTeX → DisplayList JSON (browser)
│
├── platforms/
│   ├── ios/                      # Swift + XCFramework + CoreGraphics
│   ├── android/                  # Kotlin + AAR + JNI/Canvas
│   ├── flutter/                  # Dart FFI + widget
│   ├── react-native/             # Native module + iOS/Android views
│   └── web/                      # npm package `ratex-wasm`: WASM + TypeScript web-render
│
├── tools/                        # Dev / comparison scripts
│   ├── convert_metrics.py        # KaTeX fontMetricsData.js → Rust
│   ├── convert_symbols.py        # KaTeX symbols.js → Rust
│   ├── golden_compare/           # Golden PNG comparison (compare_golden.py)
│   ├── layout_compare/            # Layout box vs KaTeX (katex_layout.mjs + compare_layouts.py)
│   ├── lexer_compare/             # Token output vs KaTeX lexer
│   └── parser_compare/            # Parser comparison
│
├── tests/
│   └── golden/                   # Golden test assets
│       ├── fixtures/              # KaTeX reference PNGs (per test case)
│       ├── output/                # RaTeX-rendered PNGs (from ratex-render)
│       └── test_cases.txt         # One LaTeX formula per line
│
├── scripts/
│   └── update_golden_output.sh    # Renders all test_cases.txt → output/
│
└── demo/                         # Web demo + sample apps (web, ios, android, flutter, RN)
```

---

## Cargo.toml (Workspace)

```toml
[workspace]
resolver = "2"
members = [
    "crates/ratex-types",
    "crates/ratex-font",
    "crates/ratex-lexer",
    "crates/ratex-parser",
    "crates/ratex-layout",
    "crates/ratex-ffi",
    "crates/ratex-render",
    "crates/ratex-wasm",
]

[workspace.package]
version = "0.0.10"   # bump with VERSION + scripts/set-version.sh; see RELEASING.md
edition = "2021"
authors = ["RaTeX Contributors"]
license = "MIT"

[workspace.dependencies]
ratex-types  = { path = "crates/ratex-types" }
ratex-font   = { path = "crates/ratex-font" }
ratex-lexer  = { path = "crates/ratex-lexer" }
ratex-parser = { path = "crates/ratex-parser" }
ratex-layout = { path = "crates/ratex-layout" }

phf        = { version = "0.11", features = ["macros"] }
thiserror  = "1.0"
serde      = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## Crates (summary)

| Crate | Role |
|-------|------|
| **ratex-types** | `DisplayList`, `DisplayItem` (GlyphPath, Line, Rect, Path), `Color`, `PathCommand`, `MathStyle` |
| **ratex-font** | KaTeX font metrics, symbol tables; `data/metrics_data.rs`, `data/symbols_data.rs` (generated) |
| **ratex-lexer** | LaTeX string → token stream |
| **ratex-parser** | Token stream → ParseNode AST (macro expansion, functions) |
| **ratex-layout** | AST → LayoutBox tree → `to_display_list` → DisplayList |
| **ratex-ffi** | C ABI: `ratex_parse_and_layout` → DisplayList JSON; Android `jni` module when targeting Android |
| **ratex-render** | DisplayList → PNG via tiny-skia + ab_glyph (server/CI) |
| **ratex-wasm** | WASM: parse + layout → DisplayList JSON for browser |

---

## ratex-types — DisplayItem (actual shape)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DisplayItem {
    GlyphPath {
        x: f64, y: f64,
        scale: f64,
        font: String,
        char_code: u32,
        commands: Vec<PathCommand>,
        color: Color,
    },
    Line { x: f64, y: f64, width: f64, thickness: f64, color: Color },
    Rect { x: f64, y: f64, width: f64, height: f64, color: Color },
    Path {
        x: f64, y: f64,
        commands: Vec<PathCommand>,
        fill: bool,
        color: Color,
    },
}
```

---

## ratex-font layout

```
crates/ratex-font/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── font_id.rs       # FontId enum
    ├── metrics.rs       # CharMetrics, math constants
    ├── symbols.rs       # Symbol lookup
    └── data/            # Generated (do not edit by hand)
        ├── mod.rs
        ├── metrics_data.rs
        └── symbols_data.rs
```

---

## ratex-ffi

Exports a C ABI used by iOS (static lib / XCFramework), Android (JNI), Flutter (Dart FFI), and React Native (native module). Main entry: parse LaTeX and return a heap-allocated JSON `DisplayList` string; callers free with `ratex_free_display_list`. On failure, use `ratex_get_last_error`. See crate-level docs in `crates/ratex-ffi/src/lib.rs`.

---

## ratex-render layout

```
crates/ratex-render/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main.rs          # CLI binary (stdin → PNGs)
│   └── renderer.rs      # DisplayList → tiny-skia rasterize
└── tests/
    └── golden_test.rs   # Compares output/ vs fixtures/ (ink score)
```

---

## ratex-wasm

WASM crate; exports `renderLatex(latex: string) => string` (DisplayList JSON). Consumed by `platforms/web` (TypeScript + Canvas 2D).

---

## Dependency graph

```
ratex-types (base types)
    ↑
ratex-font (metrics + symbols)
    ↑
ratex-lexer
    ↑
ratex-parser
    ↑
ratex-layout
    ↑
    ├── ratex-ffi    (C ABI for native)
    ├── ratex-render (PNG)
    └── ratex-wasm   (browser JSON)
    ↑
platforms/ (ios, android, flutter, react-native, web)
```

---

## Golden test workflow

1. **Reference PNGs**: `tests/golden/fixtures/` (from KaTeX, one per line in `test_cases.txt`).
2. **RaTeX output**: `scripts/update_golden_output.sh` runs `ratex-render` to produce `tests/golden/output/*.png`.
3. **Comparison**: `tools/golden_compare/compare_golden.py` (or Rust test `crates/ratex-render/tests/golden_test.rs`) compares output vs fixtures (e.g. ink-coverage threshold).

See also `docs/LOW_SCORE_CASES.md` for low-scoring cases and `docs/KATEX_SVG_PATH_PLAN.md` for stretchy SVG path improvements. Contributing: root `CONTRIBUTING.md`; releases: `RELEASING.md`.
