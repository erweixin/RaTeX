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
│   ├── ratex-katex-fonts/        # KaTeX TTF blobs for embed-fonts (crates.io–safe path)
│   ├── ratex-ffi/                # C ABI: LaTeX → DisplayList JSON (+ Android JNI)
│   ├── ratex-render/             # DisplayList → PNG (tiny-skia, server-side)
│   ├── ratex-wasm/               # WASM: LaTeX → DisplayList JSON (browser)
│   ├── ratex-svg/                # SVG export: DisplayList → SVG string (vector output)
│   ├── ratex-pdf/                # PDF export: DisplayList → PDF (pdf-writer, embedded fonts)
│   └── ratex-unicode-font/       # System Unicode / CJK font discovery for fallback rendering
│
├── platforms/
│   ├── ios/                      # Swift + XCFramework + CoreGraphics
│   ├── android/                  # Kotlin + AAR + JNI/Canvas
│   ├── flutter/                  # Dart FFI + widget
│   ├── react-native/             # Native module + iOS/Android views
│   └── web/                      # npm package `ratex-wasm`: WASM + TypeScript web-render
│
├── tools/                        # Dev / comparison scripts
│   ├── mhchem_reference.js       # KaTeX mhchem.js reference; → data/*.json via generate_mhchem_data.mjs
│   ├── generate_mhchem_data.mjs  # Export machines.json + patterns_regex.json (see docs/MHCHEM_DATA.md)
│   ├── dump_mhchem_structure.mjs # Optional: state machine stats dump
│   ├── extract_mhchem_manual_examples.mjs  # gh-pages manual → tests/golden/test_case_ce.txt
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
│       ├── fixtures_ce/           # KaTeX+mhchem reference PNGs (optional; for test_case_ce)
│       ├── output/                # RaTeX-rendered PNGs (from ratex-render)
│       ├── output_ce/             # RaTeX mhchem renders (from update_golden_output.sh)
│       ├── test_cases.txt         # One LaTeX formula per line
│       ├── test_case_ce.txt       # mhchem \\ce / \\pu examples (fixtures_ce/ refs); parser uses Rust mhchem
│
├── scripts/
│   ├── set-version.sh             # Sync version to all platform manifests
│   ├── sync-katex-ttf-to-font-crate.sh  # Copy KaTeX *.ttf → crates/ratex-katex-fonts/fonts/
│   └── update_golden_output.sh    # Renders all test_cases.txt → output/
│
└── demo/                         # Web demo + sample apps (web, ios, android, flutter, RN, jvm)
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
    "crates/ratex-katex-fonts",
    "crates/ratex-ffi",
    "crates/ratex-render",
    "crates/ratex-svg",
    "crates/ratex-wasm",
    "crates/ratex-pdf",
    "crates/ratex-unicode-font",
]

[workspace.package]
version = "0.0.16"   # 与根目录 VERSION 及 scripts/set-version.sh 同步；见 RELEASING.md
edition = "2021"
authors = ["RaTeX Contributors"]
license = "MIT"
repository = "https://github.com/erweixin/RaTeX"
homepage = "https://github.com/erweixin/RaTeX"
documentation = "https://github.com/erweixin/RaTeX#readme"

[workspace.dependencies]
# 节选：各 ratex-* crate 使用 path + 与 workspace 对齐的 version；完整依赖表见仓库根 Cargo.toml
ratex-types  = { path = "crates/ratex-types", version = "0.0.16" }
ratex-font   = { path = "crates/ratex-font", version = "0.0.16" }
# …

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
| **ratex-katex-fonts** | Bundled KaTeX `.ttf` files + embed API; optional dep for `ratex-svg` / `ratex-render` / `ratex-pdf` `embed-fonts` |
| **ratex-ffi** | C ABI: `ratex_parse_and_layout` → DisplayList JSON; Android `jni` module when targeting Android |
| **ratex-render** | DisplayList → PNG via tiny-skia + ab_glyph (server/CI); `embed-fonts` uses `ratex-katex-fonts` |
| **ratex-wasm** | WASM: parse + layout → DisplayList JSON for browser |
| **ratex-svg** | SVG export: DisplayList → SVG string; `standalone` reads TTF from `font_dir`; `embed-fonts` uses `ratex-katex-fonts`; `cli` adds `render-svg` binary |
| **ratex-pdf** | PDF export: DisplayList → PDF via [pdf-writer](https://docs.rs/pdf-writer) + font subsetting; `embed-fonts` uses `ratex-katex-fonts`; `cli` adds `render-pdf` binary |
| **ratex-unicode-font** | System Unicode / CJK font discovery; used by `ratex-render`, `ratex-svg`, `ratex-pdf` for fallback rendering of CJK / emoji / other glyphs absent from KaTeX font set |

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

## ratex-svg

SVG export crate. Converts a `DisplayList` into an SVG string via `render_to_svg(list, opts)`.

```
crates/ratex-svg/
├── Cargo.toml
└── src/
    ├── lib.rs           # render_to_svg + SvgOptions; GlyphPath→<text>, Line/Rect→<rect>, Path→<path>
    ├── standalone.rs    # (feature=standalone) load KaTeX TTF, convert glyph outlines to <path> data
    └── bin/
        └── render_svg.rs  # CLI binary (feature=cli): stdin LaTeX → SVG files
```

**Features:**

| Feature | Description |
|---------|-------------|
| `standalone` | Embed glyph outlines as `<path>` using `ab_glyph` (requires KaTeX TTF files). Produces self-contained SVGs with no external font dependency. |
| `cli` | Enables the `render-svg` binary (implies `standalone` + pulls in `ratex-layout` / `ratex-parser`). |

**`SvgOptions` fields:** `font_size` (em units, default 40.0), `padding` (default 10.0), `stroke_width` (default 1.5), `embed_glyphs` (use `<path>` outlines), `font_dir` (KaTeX TTF directory for standalone mode).

---

## ratex-pdf

PDF export crate. Converts a `DisplayList` to PDF bytes via `render_to_pdf(list, &PdfOptions)`.

```
crates/ratex-pdf/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── fonts.rs     # load KaTeX TTF (disk or embed-fonts), subset, embed CIDFontType2
    ├── renderer.rs  # content stream, paths, text
    └── bin/
        └── render_pdf.rs  # CLI binary (feature=cli): stdin LaTeX → one PDF per line
```

**Features:**

| Feature | Description |
|---------|-------------|
| `embed-fonts` | Load TTFs from `ratex-katex-fonts` (no on-disk `font_dir` required). |
| `cli` | Enables the `render-pdf` binary (implies `embed-fonts` + `ratex-layout` / `ratex-parser`). The CLI’s `--font-dir` flag does not affect embedding (same as any `embed-fonts` build). |

**`PdfOptions` fields:** `font_size`, `padding`, `stroke_width`, `font_dir` (KaTeX TTF directory when **`embed-fonts` is off**; must be set — `PdfOptions::default` uses an empty `font_dir`. When `embed-fonts` is on, `font_dir` is ignored.)

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
    ├── ratex-ffi          (C ABI for native)
    ├── ratex-render ─┐    (PNG)
    ├── ratex-wasm    │    (browser JSON)
    ├── ratex-svg     ├── ratex-unicode-font (CJK fallback loader)
    └── ratex-pdf     ┘    (PDF)
    ↑
platforms/ (ios, android, flutter, react-native, web)
```

---

## Golden test workflow

1. **Reference PNGs**: `tests/golden/fixtures/` (from KaTeX, one per line in `test_cases.txt`).
2. **RaTeX output**: `scripts/update_golden_output.sh` runs `ratex-render` to produce `tests/golden/output/*.png`.
3. **Comparison**: `tools/golden_compare/compare_golden.py` (or Rust test `crates/ratex-render/tests/golden_test.rs`) compares output vs fixtures (e.g. ink-coverage threshold).

See also `docs/MHCHEM_DATA.md` (updating `\ce` / `\pu` JSON from KaTeX mhchem). Contributing: root `CONTRIBUTING.md`; releases: `RELEASING.md`.
