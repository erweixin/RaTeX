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
| Syntax coverage | 100% | ~100% | **Aligned with KaTeX math syntax** |

<sub>\* Embeddable in non-web targets only by hosting a WebView or headless browser, which most native and server contexts can't tolerate.</sub>

**On the web specifically**, KaTeX has a decade of V8 JIT optimization behind it and remains the obvious choice for web-only projects. RaTeX's contribution isn't beating it on its home turf — it's being the only KaTeX-compatible engine that runs natively on every *other* surface, with pixel-identical output across all of them.

---

## What it renders

**Math** — **Aligned with KaTeX’s math syntax**: fractions, radicals, integrals, matrices, environments, stretchy delimiters, and more. The small set of DOM / trust-related extensions (e.g. `\includegraphics`, `\htmlClass`, …) is documented under *KaTeX differences (commands & DOM)* below.

**Chemistry** — full mhchem support via `\ce` and `\pu`:

```latex
\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}
\ce{Fe^{2+} + 2e- -> Fe}
\pu{1.5e-3 mol//L}
```

**Physics units** — `\pu` for value + unit expressions following IUPAC conventions.

**Proof trees** — bussproofs-style `prooftree` for inference rules and sequent calculi:

```latex
\begin{prooftree}
\AxiomC{A \fCenter B}
\LeftLabel{cut}
\RightLabel{\alpha}
\UnaryInfC{C \fCenter D}
\end{prooftree}
```

Supported commands include `\AxiomC` / `\AXC`, unary through quinary inference commands (`\UnaryInfC`, `\BinaryInfC`, … and abbreviations such as `\UIC`, `\BIC`, `\TIC`), `\LeftLabel` / `\RightLabel` (`\LL` / `\RL`), `\solidLine` / `\singleLine`, `\dashedLine`, `\noLine`, the corresponding `\always*Line` defaults, `\rootAtTop` / `\rootAtBottom`, `\alwaysRootAtTop` / `\alwaysRootAtBottom`, and `\fCenter`.

Not yet implemented from bussproofs: `\InsertBetweenHyps`, `\ScoreTree`, `\Cell`, and `\noCell`. Proof-tree golden references use MathJax’s bussproofs extension because KaTeX does not implement `prooftree`.

### KaTeX differences (commands & DOM)

These are the **command-level** gaps vs KaTeX (including `trust`-style HTML). Typical math and mhchem inputs are aligned with KaTeX. Some formulas still score below 1.0 in the [support table](https://erweixin.github.io/RaTeX/demo/support-table.html) or golden ink comparison due to layout/metrics/rasterization vs reference PNGs — that is **not** the same as “missing syntax” in the table below.

| KaTeX input | Notes |
|-------------|------|
| `\includegraphics[…]{…}` | **Not supported:** parser has no handler (undefined control sequence). |
| `\htmlClass`, `\htmlData`, `\htmlId` | **Not equivalent:** expanded as macros that **drop** the first argument’s `class` / `data-*` / `id` and keep only the second-argument body (unlike KaTeX trusted DOM attributes). |
| `\htmlStyle{…}{…}` | **Partial:** simple inline styling may work on Web/Canvas paths; behavior may still differ from KaTeX’s DOM-based HTML extension. |

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
| **Server / CI** | `ratex-render` → tiny-skia PNG rasterizer | Out of the box |
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
        C[ratex-parser\nmhchem · numbering · bussproofs]
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
| `ratex-parser` | Token stream → ParseNode AST; mhchem `\ce` / `\pu`; bussproofs `prooftree`; auto-numbering for `equation` / `align` / `gather` / `alignat` and end-of-row `\tag` / `\nonumber` / `\notag` |
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

### Render to PNG · SVG · PDF

Prebuilt binaries:

[GitHub Releases](https://github.com/erweixin/RaTeX/releases) provides prebuilt CLI archives. Select the archive that matches the target operating system and CPU architecture, then extract it. The prebuilt binaries bundle KaTeX fonts, so `--font-dir` is not required.

Build the CLI binaries from source:

```bash
cargo build --release -p ratex-render
cargo build --release -p ratex-svg --features "cli standalone"
cargo build --release -p ratex-pdf --features cli
```

| Output | Crate / Binary | Build |
| --- | --- | --- |
| PNG | `ratex-render` / `render` | `cargo build --release -p ratex-render`; optionally add `--features embed-fonts` |
| SVG | `ratex-svg` / `render-svg` | `cargo build --release -p ratex-svg --features "cli standalone"`; or `--features "cli embed-fonts"` |
| PDF | `ratex-pdf` / `render-pdf` | `cargo build --release -p ratex-pdf --features cli`; or `--features "cli embed-fonts"` |

Build notes:

1. Without `embed-fonts`, all three CLIs first search the default KaTeX TTF locations; pass `--font-dir` only if your fonts are elsewhere. With `embed-fonts`, KaTeX TTFs are bundled via the [`ratex-katex-fonts`](crates/ratex-katex-fonts) crate, so `--font-dir` is no longer needed. After upgrading KaTeX fonts, run [`scripts/sync-katex-ttf-to-font-crate.sh`](scripts/sync-katex-ttf-to-font-crate.sh) to refresh the bundled font crate.
2. The current `render-svg` CLI always builds self-contained output in `standalone` mode with `embed_glyphs = true`. As a library, `ratex-svg` still defaults to `SvgOptions::embed_glyphs = false`, which emits `<text>` elements that rely on KaTeX CSS/webfonts.

Examples:

```bash
# These examples use `printf '%s\n' '...'` because different shells handle
# backslashes in `echo` differently; the same formula may need `\frac` in one
# shell and `\\frac` in another.

# PNG: read directly from stdin
printf '%s\n' '\frac{1}{2} + \sqrt{x}' | ./target/release/render --output-dir ./out

# SVG: load KaTeX TTFs at runtime
printf '%s\n' '\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}' | \
  ./target/release/render-svg --font-dir /path/to/katex/fonts --color '#1E88E5' --output-dir ./out

# SVG: write the rendered SVG document to stdout instead of a file
printf '%s\n' '\frac{1}{2} + \sqrt{x}' | ./target/release/render-svg --stdout > formula.svg

# PDF: load KaTeX TTFs at runtime
printf '%s\n' '\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}' | \
  ./target/release/render-pdf --font-dir /path/to/katex/fonts --output-dir ./out

# You can also read from a file via stdin
cat formulas.txt | ./target/release/render --output-dir ./out

# Or pass the input file directly
./target/release/render-svg --input formulas.txt --font-dir /path/to/katex/fonts --output-dir ./out
```

CLI notes

- `--input <FILE>`: read formulas from a file, one per line.
- `--output-dir <DIR>`: output directory. Defaults are `output`, `output_svg`, and `output_pdf`.
- `render-svg --stdout`: write SVG documents to stdout instead of files; status messages go to stderr.
- `--help`: show supported options and whether the current build uses embedded fonts.
- `--color` / `--background-color` accept named colors (for example `black`, `red`, `teal`), 3- or 6-digit hex (`#f00`, `#ff0000`), and KaTeX / MathJax-style color model values (`[RGB]255,0,0`, `[rgb]1,0,0`, `[HTML]B22222`, `[gray]0.5`, `[cmyk]0,1,1,0`).
- `ratex-render --background-color transparent` produces a transparent PNG.

### CJK / Unicode fallback

By default RaTeX bundles only KaTeX fonts (19 faces for math symbols). Characters outside the KaTeX glyph set — CJK ideographs, emoji, Hangul, etc. — are rendered via a system Unicode font discovered automatically:

1. **`RATEX_UNICODE_FONT`** env var — path to any `.ttf`/`.otf`/`.ttc`, with optional `#index` or `#FamilyName` selector for TTC collections (e.g. `NotoSansCJK.ttc#Noto Sans CJK SC`)
2. **Hard-coded system paths** — Linux (`/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc`), macOS (`/Library/Fonts/Arial Unicode.ttf`, `/System/Library/Fonts/Supplemental/Arial Unicode.ttf`), Windows (`C:\Windows\Fonts\NotoSansSC-VF.ttf`, `C:\Windows\Fonts\msyh.ttc`)
3. **Locale-aware system discovery** — `system-fonts` resolves prioritized Sans candidates for the current system locale / region, including TTC family selection when needed

```bash
# Explicit font path (recommended for CI / server environments)
printf '%s\n' '\text{你好世界}' | \
  RATEX_UNICODE_FONT=/path/to/NotoSansSC-Regular.ttf ./target/release/render --output-dir ./out

# Auto-discovery probes built-in paths first, then locale-aware system Sans fallbacks.
printf '%s\n' '\text{你好世界}' | ./target/release/render-pdf --output-dir ./out
```

All three renderers (PNG, SVG, PDF) use the same discovery crate (`ratex-unicode-font`), so once a font is found the output is consistent across all formats. For variable fonts, RaTeX prefers the Regular `wght=400` instance when that axis is available so outline extraction, metrics, and PDF subsetting stay aligned. For PNG and standalone SVG, glyph outlines are embedded as paths. For PDF, the detected CJK glyphs are subsetted and embedded as a CIDFontType2 font.

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

RaTeX follows KaTeX-style layout for numbered display environments.

- **Auto-numbering** applies to non-starred `equation`, `align`, `alignat`, and `gather`: each logical row gets a sequential tag such as `(1)`, `(2)`, … . Starred forms (`equation*`, `align*`, …) and inner environments **`aligned`**, **`alignedat`**, **`split`**, and **`gathered`** do **not** auto-number (same idea as LaTeX: only the outer display is numbered).
- **`\tag{...}`** / **`\tag*{...}`** at the **end** of a row replace the auto number for that row (amsmath-style). Empty `\tag{}` suppresses the number for that row.
- **`\nonumber`** and **`\notag`** at the **end** of a row suppress the number for that row when auto-numbering is active. They cannot be combined with `\tag` on the same row.
- **`\notag`** is implemented as an alias of **`\nonumber`** (same as above).

Document-level options such as `\leqno` and cross-reference counters are not modeled; numbering starts from `(1)` within the parse of each formula string.

---

## Acknowledgements

RaTeX owes a great debt to [KaTeX](https://katex.org/) — its parser architecture, symbol tables, font metrics, and layout semantics are the foundation of this engine. Chemistry notation (`\ce`, `\pu`) is powered by a Rust port of the [mhchem](https://mhchem.github.io/MathJax-mhchem/) state machine.

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). To report a security issue, see [`SECURITY.md`](SECURITY.md).

---

## License

MIT — Copyright (c) erweixin.
