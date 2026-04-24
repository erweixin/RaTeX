# RaTeX

[简体中文](README.zh-CN.md) | **English**

**KaTeX-compatible math rendering engine in pure Rust — no JavaScript, no WebView, no DOM.**

One Rust core, one display list, every platform renders natively.

```
\frac{-b \pm \sqrt{b^2-4ac}}{2a}   →   iOS · Android · Flutter · React Native · Web · PNG · SVG · PDF
```

**[→ Live Demo](https://erweixin.github.io/RaTeX/demo/live.html)** — type LaTeX and compare RaTeX vs KaTeX side-by-side ·
**[→ Support table](https://erweixin.github.io/RaTeX/demo/support-table.html)** — RaTeX vs KaTeX across all test formulas ·
**[→ Web benchmark](https://erweixin.github.io/RaTeX/demo/benchmark.html)** — head-to-head perf in the browser

---

## Why RaTeX?

Every major cross-platform math renderer today runs LaTeX through a browser or JavaScript engine — a hidden WebView eating 50–150 MB RAM, startup latency before the first formula, no offline guarantee. KaTeX is excellent on the web, but on every other surface — iOS, Android, Flutter, server-side, embedded — you're either hosting a WebView or shelling out to headless Chrome.

RaTeX is the same KaTeX-compatible math engine compiled to a portable Rust core, so the *same* renderer runs natively everywhere — and produces byte-identical output across every target.

| | KaTeX | MathJax | **RaTeX** |
|---|---|---|---|
| Runtime | JS (V8) | JS (V8) | **Pure Rust** |
| Surfaces it runs on | Web only* | Web only* | **iOS · Android · Flutter · RN · Web · server · SVG · PDF** |
| Mobile | WebView wrapper | WebView wrapper | **Native** |
| Server-side rendering | headless Chrome | mathjax-node | **Single binary, no JS runtime** |
| Output substrate | DOM (`<span>` tree) | DOM / SVG | **Display list → Canvas / PNG / SVG / PDF** |
| Memory | GC / heap | GC / heap | **Predictable, no GC** |
| Offline | Depends | Depends | **Yes** |
| Syntax coverage | 100% | ~100% | **~99%** |

<sub>\* Embeddable in non-web targets only by hosting a WebView or headless browser, which most native and server contexts can't tolerate.</sub>

**On the web specifically**, KaTeX has a decade of V8 JIT optimization behind it and remains the obvious choice for web-only projects. RaTeX's contribution isn't beating it on its home turf — it's being the only KaTeX-compatible engine that runs natively on every *other* surface, with pixel-identical output across all of them.

---

## What it renders

**Math** — ~99% of KaTeX syntax: fractions, radicals, integrals, matrices, environments, stretchy delimiters, and more.

**Chemistry** — full mhchem support via `\ce` and `\pu`:

```latex
\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}
\ce{Fe^{2+} + 2e- -> Fe}
\pu{1.5e-3 mol//L}
```

**Physics units** — `\pu` for value + unit expressions following IUPAC conventions.

---

## Platform targets

| Platform | How | Status |
|---|---|---|
| **iOS** | XCFramework + Swift / CoreGraphics | Out of the box |
| **Android** | JNI + Kotlin + Canvas · AAR | Out of the box |
| **Flutter** | Dart FFI + `CustomPainter` | Out of the box |
| **React Native** | Native module + C ABI · iOS/Android views | Out of the box |
| **Compose Multiplatform** | Kotlin Multiplatform + Compose Canvas · Android / iOS / JVM Desktop | Via [`RaTeX-CMP`](https://github.com/darriousliu/RaTeX-CMP) |
| **Web** | WASM → Canvas 2D · `<ratex-formula>` Web Component | Out of the box |
| **Server / CI** | tiny-skia → PNG rasterizer | Out of the box |
| **SVG** | `ratex-svg` → self-contained vector SVG | Out of the box |
| **PDF** | `ratex-pdf` → vector PDF with embedded KaTeX fonts | Out of the box |

### Screenshots

From the demo apps in [`demo/screenshots/`](demo/screenshots/).

<table>
  <tr>
    <th width="50%">iOS</th>
    <th width="50%">Android</th>
  </tr>
  <tr>
    <td align="center"><img alt="RaTeX demo on iOS" src="demo/screenshots/ios.png" width="100%"/></td>
    <td align="center"><img alt="RaTeX demo on Android" src="demo/screenshots/android.png" width="100%"/></td>
  </tr>
  <tr>
    <th width="50%">Flutter (iOS)</th>
    <th width="50%">React Native (iOS)</th>
  </tr>
  <tr>
    <td align="center"><img alt="RaTeX demo on Flutter iOS" src="demo/screenshots/flutter-ios.png" width="100%"/></td>
    <td align="center"><img alt="RaTeX demo on React Native iOS" src="demo/screenshots/react-native-ios.png" width="100%"/></td>
  </tr>
  <tr>
    <th colspan="2">Compose Multiplatform</th>
  </tr>
  <tr>
    <td colspan="2" align="center"><img alt="RaTeX demo on Compose Multiplatform" src="demo/screenshots/compose-multiplatform.png" width="100%"/></td>
  </tr>
</table>

---

## Architecture

```mermaid
flowchart LR
    A["LaTeX string\n(math · \\ce · \\pu)"]
    subgraph core["Rust core"]
        B[ratex-lexer]
        C[ratex-parser\nmhchem \\ce / \\pu]
        D[ratex-layout]
        E[DisplayList]
    end
    F[ratex-ffi\niOS · Android · Flutter · RN]
    G[ratex-wasm\nWeb / Canvas 2D]
    H[ratex-render\nPNG · tiny-skia]
    I[ratex-svg\nSVG]
    J[ratex-pdf\nPDF]
    K[ratex-unicode-font\nCJK fallback loader]
    A --> B --> C --> D --> E
    E --> F
    E --> G
    E --> H
    E --> I
    E --> J
    H -.-> K
    I -.-> K
    J -.-> K
```

| Crate | Role |
|---|---|
| `ratex-types` | Shared types: `DisplayItem`, `DisplayList`, `Color`, `MathStyle` |
| `ratex-font` | KaTeX-compatible font metrics and symbol tables |
| `ratex-lexer` | LaTeX → token stream |
| `ratex-parser` | Token stream → ParseNode AST; includes mhchem `\ce` / `\pu` |
| `ratex-layout` | AST → LayoutBox tree → DisplayList |
| `ratex-ffi` | C ABI: exposes the full pipeline for native platforms |
| `ratex-wasm` | WASM: pipeline → DisplayList JSON for the browser |
| `ratex-render` | Server-side: DisplayList → PNG (tiny-skia) |
| `ratex-svg` | SVG export: DisplayList → SVG string |
| `ratex-pdf` | PDF export: DisplayList → PDF bytes ([pdf-writer](https://docs.rs/pdf-writer), embedded CID fonts) |
| `ratex-unicode-font` | System Unicode / CJK font discovery for fallback rendering |

---

## Quick start

**Requirements:** Rust 1.70+ ([rustup](https://rustup.rs))

```bash
git clone https://github.com/erweixin/RaTeX.git
cd RaTeX
cargo build --release
```

### Render to PNG

```bash
echo '\frac{1}{2} + \sqrt{x}' | cargo run --release -p ratex-render -- --color '#1E88E5'

echo '\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}' | cargo run --release -p ratex-render
```

### Render to SVG

```bash
# Default: glyphs as <text> elements (correct display requires KaTeX webfonts)
echo '\frac{1}{2} + \sqrt{x}' | cargo run --release -p ratex-svg --features cli -- --color '#1E88E5'

# Standalone: embed glyph outlines as <path> — no external fonts needed
echo '\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}' | \
  cargo run --release -p ratex-svg --features "cli embed-fonts" -- \
  --output-dir ./out
```

The `standalone` feature (enabled by `cli`) reads KaTeX TTF files from `--font-dir` and embeds glyph outlines directly into the SVG, producing a fully self-contained file that renders correctly without any CSS or web fonts.

The `embed-fonts` feature (implicitly enables `standalone`) bundles the same TTFs via the [`ratex-katex-fonts`](crates/ratex-katex-fonts) crate, so no `--font-dir` is needed and builds from crates.io stay self-contained. To refresh bundled fonts after upgrading KaTeX, run [`scripts/sync-katex-ttf-to-font-crate.sh`](scripts/sync-katex-ttf-to-font-crate.sh).

### Render to PDF

```bash
# `cli` implies `embed-fonts`: KaTeX TTFs are bundled via ratex-katex-fonts (--font-dir is ignored)
echo '\frac{1}{2} + \sqrt{x}' | cargo run --release -p ratex-pdf --features cli -- --output-dir ./out

# Equivalent font loading (explicit embed-fonts)
echo '\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}' | \
  cargo run --release -p ratex-pdf --features "cli embed-fonts" -- --output-dir ./out
```

The `ratex-pdf` crate writes one `.pdf` per non-empty line from stdin. Options include `--output-dir` (default `output_pdf`), `--font-size`, `--dpr`, and `--inline` (text style instead of display). The `render-pdf` binary always loads fonts from `ratex-katex-fonts`, so `--font-dir` does not change embedding. For library use without `embed-fonts`, set `PdfOptions.font_dir` to your KaTeX TTF directory instead.

### CJK / Unicode fallback

By default RaTeX bundles only KaTeX fonts (19 faces for math symbols). Characters outside the KaTeX glyph set — CJK ideographs, emoji, Hangul, etc. — are rendered via a system Unicode font discovered automatically:

1. **`RATEX_UNICODE_FONT`** env var — explicit path to any `.ttf`/`.otf`
2. **Hard-coded system paths** — macOS (e.g. `/Library/Fonts/Supplemental/Arial Unicode.ttf`, `/System/Library/Fonts/Supplemental/Arial Unicode.ttf`), Linux (`/usr/share/fonts/…`), Windows (`C:\Windows\Fonts\…`)
3. **fontdb system query** — SansSerif scan, then brute-force

```bash
# Explicit font path (recommended for CI / server environments)
RATEX_UNICODE_FONT=/path/to/NotoSansSC-Regular.ttf \
  echo '\text{你好世界}' | cargo run --release -p ratex-render

# Auto-discovery finds Arial Unicode on macOS, DejaVu Sans on Linux, etc.
echo '\text{你好世界}' | cargo run --release -p ratex-render
```

All three renderers (PNG, SVG, PDF) use the same discovery crate (`ratex-unicode-font`), so once a font is found the output is consistent across all formats. For PNG and standalone SVG, glyph outlines are embedded as paths. For PDF, the detected CJK glyphs are subsetted and embedded as a CIDFontType2 font.

### Browser (WASM)

```bash
npm install ratex-wasm
```

```html
<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
<script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>

<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" color="#1E88E5"></ratex-formula>
<ratex-formula latex="\ce{CO2 + H2O <=> H2CO3}" font-size="32"></ratex-formula>
```

See [`platforms/web/README.md`](platforms/web/README.md) for the full setup.

### Platform glue layers

| Platform | Docs |
|---|---|
| iOS | [`platforms/ios/README.md`](platforms/ios/README.md) |
| Android | [`platforms/android/README.md`](platforms/android/README.md) |
| Flutter | [`platforms/flutter/README.md`](platforms/flutter/README.md) |
| React Native | [`platforms/react-native/README.md`](platforms/react-native/README.md) |
| Compose Multiplatform | [`RaTeX-CMP`](https://github.com/darriousliu/RaTeX-CMP) |
| Web | [`platforms/web/README.md`](platforms/web/README.md) |

### Run tests

```bash
cargo test --all
```

---

## Equation numbering and `\tag`

RaTeX supports **automatic equation numbering** for AMS display environments:

- Non-starred environments (`equation`, `align`, `gather`, `alignat`) auto-generate numbers in the tag column.
- Starred forms (`equation*`, `align*`, etc.) and inner environments (`aligned`, `split`, `gathered`, `alignedat`) do not generate numbers.
- Explicit `\tag{...}` / `\tag*{...}` can be used to override or add custom tags.
- `\notag` / `\nonumber` suppress auto-numbering on a specific row.
- `\label{name}` associates the current equation number with a label.
- `\ref{name}` renders the equation number for a label (requires `external_labels` from a prior render pass).
- `\eqref{name}` renders the equation number wrapped in parentheses.

### Equation state

Auto-numbering requires an `EquationState` passed via `LayoutOptions`:

```rust
use std::cell::RefCell;
use std::rc::Rc;
use ratex_layout::{layout, EquationState, LayoutOptions};

let eq_state = Rc::new(RefCell::new(EquationState::default()));
let opts = LayoutOptions {
    equation_state: Some(eq_state.clone()),
    ..LayoutOptions::default()
};
let lbox = layout(&parse("\\begin{equation} E=mc^2 \\end{equation}").unwrap(), &opts);

// Counter was incremented
let s = eq_state.borrow();
assert_eq!(s.counter, 2);  // started at 1, was 1 after first numbered row
```

When `LayoutOptions::default()` is used (no `equation_state`), auto-numbering is silently skipped — the formula renders without equation numbers.

### Two-pass rendering for `\label` / `\ref`

`\ref` and `\eqref` resolve labels from `EquationState::external_labels`. A typical two-pass workflow:

```rust
// Pass 1: render and collect labels
let state = Rc::new(RefCell::new(EquationState::default()));
let opts = LayoutOptions {
    equation_state: Some(state.clone()),
    ..Default::default()
};
layout(&parse("\\begin{equation} a=1 \\label{eq:a} \\end{equation}").unwrap(), &opts);
let labels: HashMap<String, usize> = state.borrow().labels.clone();

// Pass 2: resolve \ref using collected labels
let state2 = Rc::new(RefCell::new(EquationState {
    external_labels: labels,
    ..EquationState::default()
}));
let opts2 = LayoutOptions {
    equation_state: Some(state2.clone()),
    ..Default::default()
};
layout(&parse("\\ref{eq:a} + \\eqref{eq:a}").unwrap(), &opts2);
```

---

## Acknowledgements

RaTeX owes a great debt to [KaTeX](https://katex.org/) — its parser architecture, symbol tables, font metrics, and layout semantics are the foundation of this engine. Chemistry notation (`\ce`, `\pu`) is powered by a Rust port of the [mhchem](https://mhchem.github.io/MathJax-mhchem/) state machine.

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). To report a security issue, see [`SECURITY.md`](SECURITY.md).

---

## License

MIT — Copyright (c) erweixin.
