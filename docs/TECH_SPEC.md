# Cross-Platform LaTeX Math Typesetting Engine — Technical Specification Summary

> Rust core engine + iOS / Android / Web rendering

---

## Document scope

This file is a **design and rationale** summary (historical estimates, KaTeX mapping, architecture). For **what exists today**—crate layout, `ratex-ffi` C ABI, npm package **`ratex-wasm`**, golden tests—see [`PROJECT_STRUCTURE.md`](./PROJECT_STRUCTURE.md) and the per-platform READMEs under `platforms/`.

---

## 1. Background and Goals

### 1.1 Starting Point: SwiftMath

SwiftMath is a pure Swift implementation of iosMath for rendering LaTeX math on iOS/macOS, using the same typesetting rules as LaTeX.

**SwiftMath code size estimate:**

| Module | Estimated LOC |
|--------|----------------|
| LaTeX parser (MTMathAtom, MTMathList, etc.) | ~3,000 |
| Typesetting engine (MTTypesetter, MTMathListDisplay) | ~4,000 |
| Font handling (MTFontManager, MTFont) | ~1,500 |
| UI layer (MTMathUILabel, platform adapters) | ~1,000 |
| Utilities, symbol tables, tests | ~2,000 |
| **Total Swift source** | **~11,000–13,000** |

### 1.2 Why a Cross-Platform Approach

- SwiftMath is tightly coupled to UIKit/AppKit and cannot be reused on Android or Web.
- Maintaining separate typesetting logic for iOS, Android, and Web is very costly.
- Rust can target native libraries (iOS/Android) and WebAssembly (Web), making it the only practical shared layer.

---

## 2. Reference Implementation Analysis

### 2.1 Why KaTeX as Reference

- KaTeX is the most complete and rigorous open-source LaTeX math parsing implementation.
- Layout strictly follows Donald Knuth's TeX standard—the de facto standard for math typesetting.
- It uses a registry-based function system (`defineFunction`) with clear boundaries per command, which maps well to a modular port.
- It has a full test suite (`ss_data.yaml`) that can validate the Rust implementation directly.

### 2.2 Core KaTeX Parser Files

| KaTeX file | Rust module | Estimated LOC |
|------------|-------------|----------------|
| `Lexer.js` | `lexer.rs` | ~400 |
| `Token.js` | `token.rs` | ~100 |
| `MacroExpander.js` | `macro_expander.rs` | ~800 (most complex) |
| `Parser.js` | `parser.rs` | ~1,500 |
| `parseNode.js` | `parse_node.rs` | ~600 |
| `symbols.js` | `symbols.rs` | ~800 |
| `defineFunction.js` + `functions/` | `functions/` (~30 files) | ~3,400 |
| `environments.js` | `environments/` | ~1,200 |
| `macros.js` | `macros.rs` | ~500 |

### 2.3 Rust Ecosystem (2025)

> ⚠️ **Conclusion: There is no stable, production-ready pure-Rust LaTeX math parser in the ecosystem. Implementing our own is the right direction.**

| Project | Status | Notes |
|---------|--------|--------|
| `pulldown-latex` | ⚠️ Largely stalled (no commits since Aug 2024) | ~7 months unmaintained |
| `latex2mathml` | ❌ Abandoned (no updates for 5 years) | Limited functionality |
| `tectonic` | ⚠️ Stalled (~2 years) | Full TeX engine, too large |
| `katex-rs` (xu-cheng) | JS wrapper | Runs KaTeX JS via QuickJS, not a real Rust implementation |

### 2.4 KaTeX-Specific Extensions (Non-Standard LaTeX)

These are KaTeX extensions for Web output and **are not standard LaTeX**. RaTeX parses them but does not implement their semantics:

| Command | Description | RaTeX behavior |
|---------|-------------|-----------------|
| `\htmlClass{class}{content}` | Add class to rendered HTML node | Parse only; expand to second argument "content"; no HTML attributes |
| `\htmlData{key=val,...}{content}` | Add data-* attributes | Same |
| `\htmlId{id}{content}` | Add id | Same |
| `\htmlStyle{css}{content}` | Add inline style | Same |

Implementation: in `macro_expander`, define them as two-argument macros; expansion is only the second argument (content); the first (attributes/selector) is consumed and discarded. No class/id/style metadata reaches rendering; formula appearance matches "without these commands".

---

## 3. Overall Architecture

### 3.1 End-to-End Data Flow

> Principle: Rust does all platform-independent computation (parse + layout); platforms only do the final pixel drawing.

| Stage | Layer | Input | Output | Notes |
|-------|--------|-------|--------|--------|
| Parse | Rust | LaTeX string | ParseNode AST | Pure math structure, no sizes or coordinates |
| Layout | Rust | ParseNode AST + font metrics | DisplayList | Absolute coordinates and sizes per element |
| Render | Platform-native | DisplayList | Pixels | ~200 lines of platform code |

```
┌─────────────────────────────────────────────────────┐
│              Rust core (cross-platform)             │
│                                                     │
│  LaTeX string                                       │
│      ↓  Lexer / MacroExpander                       │
│  ParseNode AST                                      │
│      ↓  Layout engine + static font metrics         │
│  DisplayList (draw commands with absolute coords)   │
└──────────┬──────────────┬─────────────┬─────────────┘
           │              │             │
      Swift FFI      Kotlin JNI       WASM
           │              │             │
           ▼              ▼             ▼
         iOS           Android         Web
      CoreText         Canvas       Canvas 2D
      (~200 LOC)      (~200 LOC)   (~150 LOC TS)
```

### 3.2 Why Layout Lives in Rust

- Layout depends on font metrics—static tables extracted from OTF fonts and embedded in Rust.
- No platform APIs; naturally cross-platform.
- iOS, Android, and Web share the same layout result for pixel-consistent output.
- Platform renderers stay thin (~200 LOC), translating `DisplayItem` to native calls.

### 3.3 Rust Output: DisplayList

DisplayList is the central interface—platform-agnostic draw description:

```rust
pub enum DisplayItem {
    // Draw a glyph at given coordinates
    Glyph {
        x: f64, y: f64,          // baseline (absolute)
        font_id: FontId,
        font_size: f64,          // pt size
        glyph_id: u16,           // OTF glyph index
        color: Color,
    },
    // Lines (fraction bar, radical top, underline)
    Line { x: f64, y: f64, width: f64, thickness: f64, color: Color },
    // Rectangle (\colorbox background)
    Rect { x: f64, y: f64, width: f64, height: f64, color: Color },
    // SVG path (radical corner, braces)
    Path { x: f64, y: f64, commands: Vec<PathCommand>, color: Color },
}
```

Platform consumption:

| Platform | Call | Render API | Size |
|----------|------|------------|------|
| iOS / macOS | Swift FFI (C ABI) | CoreText + CoreGraphics | ~200 LOC Swift |
| Android | Kotlin JNI | Canvas + Paint | ~200 LOC Kotlin |
| Web | WebAssembly | Canvas 2D API | ~150 LOC TypeScript |

---

## 4. Parser Layer Design

### 4.1 Parser Architecture

KaTeX's three-layer design maps directly to Rust:

```
Parser (syntax)
  └── MacroExpander / gullet (macro expansion)
        ├── stack: Vec<Token>   // replay stack, supports push_back
        └── Lexer / mouth      // read raw input when stack is empty
```

### 4.2 ParseNode AST (Parser Output)

ParseNode trees describe **math semantics** only; no sizes, coordinates, or fonts.

Example: `\frac{a^2 + b}{c}`:

```
Genfrac {
  numer: OrdGroup [
    Supsub { base: Atom('a', Ord), sup: Atom('2', Ord), sub: None },
    Atom('+', BinaryOp),
    Atom('b', Ord),
  ],
  denom: OrdGroup [ Atom('c', Ord) ],
  hasBarLine: true,
  style: Auto,
}
```

**ParseNode characteristics:**
- ✅ Describes math semantics (fraction, numerator, denominator)
- ✅ Records TeX style (display / text / script / scriptscript)
- ❌ No width/height
- ❌ No x/y coordinates

### 4.3 Key Challenges

| Challenge | Description | Approach |
|-----------|-------------|----------|
| MacroExpander borrows | Rust is strict about iterators holding mutable refs | Explicit replay stack with `Vec<Token>` |
| `\left` / `\right` pairing | Need stack for nesting | Delimiter stack during parse |
| Matrix/align environments | Columns by `&`, rows by `\\` | Collect rows first, then build Array node |
| Font variant propagation | `\mathbf` must propagate to children | Pass font context down during parse |
| Macro expansion recursion | Macros can define macros | Expansion limit (e.g. 1000) |

---

## 5. Layout Layer Design

### 5.1 TeX Box Model

Each math element is a box with three dimensions:

- **width**: horizontal extent
- **height**: above baseline (ascent)
- **depth**: below baseline (descent)

Rules: horizontal (HBox) — sum x, max height/depth; vertical (VBox) — sum y, max width. Offsets in `em`, multiplied by actual pt size.

### 5.2 Font Metrics Strategy

> 💡 **Recommended start:** Convert KaTeX's `fontMetricsData.js` to Rust static `phf_map`; correctness is already validated by KaTeX.

| Strategy | Pros | Cons | When to use |
|----------|------|------|-------------|
| A: Fixed Latin Modern Math | Simple, precise, full control | Inflexible font | Initial phase |
| B: Runtime metrics (FFI callback) | Any font | FFI overhead, complexity | Later extension |

### 5.3 Fraction Layout Example (TeX Rule 15d)

```rust
fn layout_fraction(node: &GenfracNode, style: Style) -> LayoutBox {
    let numer = layout_node(&node.numer, style.numerator_style());
    let denom = layout_node(&node.denom, style.denominator_style());

    let metrics = font_metrics(style);

    // TeX Rule 15d: different offsets for display vs text
    let (num_shift, den_shift) = if style.is_display() {
        (metrics.num1, metrics.denom1)
    } else {
        (metrics.num2, metrics.denom2)
    };

    // Clearance between numerator/denominator and bar
    let num_clearance = (num_shift - numer.depth)
                      - (metrics.axis_height + metrics.rule_thickness / 2.0);
    let num_shift = if num_clearance < MIN_CLEARANCE {
        num_shift + (MIN_CLEARANCE - num_clearance)
    } else { num_shift };

    // ... similar for denominator

    LayoutBox {
        width: numer.width.max(denom.width) + 2.0 * FRAC_PADDING,
        height: numer.height + num_shift,
        depth: denom.depth + den_shift,
        content: BoxContent::Fraction { numer, denom, num_shift, den_shift, .. },
    }
}
```

---

## 6. Implementation Order

### Phase 1: Lexer + Token (Week 1)

- Implement Lexer: LaTeX string → token stream.
- Token kinds: `Char`, `Command`, `LBrace`/`RBrace`, `Caret`, `Underscore`, `Ampersand`, `EOF`.
- Validate against KaTeX `lexer_test.js`.
- Handle edges: `\ ` space, `\\` newline, `%` comment.

### Phase 2: ParseNode Types + Symbol Table (Week 2)

- Define ~30 `ParseNode` variants (e.g. `Genfrac`, `Radical`, `Supsub`, `Op`, `Array`).
- Build symbol table with `phf_map` (500+ entries from KaTeX `symbols.js`).
- Static font metrics (from KaTeX `fontMetricsData.js`).

> 💡 Data definitions only; no parse logic. AI can batch-convert for ~10x speedup.

### Phase 3: MacroExpander + Parser Core (Weeks 3–5)

- MacroExpander: token replay stack + macro expansion.
- `parseAtom` core loop: main parsing + superscript/subscript stacking.
- Core commands first: `\frac`, `\sqrt`, `^`, `_`, `{ }`, basic atoms.
- Validate with KaTeX `ss_data.yaml` (e.g. 80% of common cases).
- Then: `\left`/`\right`, environments (matrix/align/cases), color, font commands.

### Phase 4: Layout Engine (Weeks 6–8)

- `LayoutBox` (width / height / depth + content).
- Core layout: HBox, VBox, Fraction, Radical, Supsub.
- DisplayList generation (LayoutBox tree → DisplayItem sequence with absolute coords).
- Matrix / braces / stretchy symbols.
- Validate: pixel comparison with KaTeX (see §7).

### Phase 5: FFI + Platform Renderers (Weeks 9–10)

- C ABI: `parse_latex()`, `layout()`, `free_display_list()`.
- iOS: Swift wrapper, consume DisplayItem, CoreText / CoreGraphics.
- Android: Kotlin JNI, Canvas / Paint.
- Web: WASM build, TypeScript wrapper, Canvas 2D.

---

## 7. Validation

### 7.1 Core Idea

> ⚠️ **Correctness of math typesetting cannot be auto-generated—the only reliable check is pixel-level comparison with KaTeX.**

### 7.2 Golden Test Framework

1. Render ~500 test formulas with KaTeX → reference PNGs.
2. Render same formulas with our engine → PNGs.
3. Pixel diff with tolerance (e.g. 1px for antialiasing).
4. Mark regressions and produce visual diff reports.

**Test sources:**
- KaTeX: `katex/test/screenshotter/ss_data.yaml` (~900+ formulas).
- SwiftMath tests (common iOS cases).
- Custom edge cases: long formulas, nested fractions, multi-line matrices.

### 7.3 Layered Validation

| Layer | What | Tool | When |
|-------|------|------|------|
| Lexer | Token sequence matches KaTeX | Unit tests vs KaTeX lexer | After Phase 1 |
| Parser | ParseNode structure matches KaTeX | JSON structure compare | After Phase 3 |
| Layout | LayoutBox dimensions | Compare with KaTeX HTML em values | During Phase 4 |
| Render | Pixel PNG comparison | Golden test | After Phase 4 |
| Integration | Device rendering | Manual screenshot check | After Phase 5 |

### 7.4 Value of Golden Tests

About 1 week to set up; ongoing value:

- Every PR runs it to catch regressions.
- AI-generated code is validated the same way.
- Quantified quality before release (e.g. "98.5% pass, 900 formulas").

---

## 8. Effort and AI Acceleration

### 8.1 Effort Estimate

| Module | LOC | Original estimate | With AI | AI speedup |
|--------|-----|-------------------|---------|------------|
| Symbol table / font metrics | ~1,300 | 3 d | 0.5 d | 10x |
| AST type definitions | ~600 | 2 d | 0.5 d | 5x |
| Lexer | ~400 | 2 d | 0.5 d | 4x |
| MacroExpander | ~800 | 8 d | 4 d | 2x |
| Parser core | ~2,000 | 15 d | 6 d | 2.5x |
| Layout core | ~2,500 | 18 d | 9 d | 2x |
| Matrix / Stretchy | ~1,000 | 8 d | 4 d | 2x |
| DisplayList + FFI | ~600 | 4 d | 1.5 d | 3x |
| Platform renderers (×3) | ~600 | 6 d | 2 d | 3x |
| Tests + golden framework | ~1,500 | 8 d | 4 d | 2x |
| **Total** | **~11,300** | **74 d** | **~32 d** | **~2.3x** |

### 8.2 Where AI Helps

| Scenario | AI effect | Reason |
|----------|-----------|--------|
| Symbol/metrics (JS→Rust) | ✅ Very high (10x) | Pure data, fixed patterns |
| ParseNode type definitions | ✅ High (5x) | Flow → Rust enum, mechanical |
| Command handlers | ✅ Good (3x) | Clear logic, reference available |
| MacroExpander borrows | ⚠️ Limited (2x) | AI often produces uncompilable Rust |
| TeX rule correctness | ❌ Not replaceable | Must compare pixels with KaTeX |

### 8.3 Recommended Workflow

Use AI for **ongoing pair programming**, not "generate everything at once":

```
1. Human: Understand KaTeX module (30 min)
2. AI:   Generate Rust (5 min)
3. Human: Fix borrows, compile (30 min)
4. Auto: Run pixel comparison (5 min)
5. Repeat for next module
```

> With two people (one on parser, one on layout), total time can be ~**20 days**.

---

## 9. Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Reference | KaTeX (not MathJax/iosMath) | Clear architecture, modular functions, full tests |
| Rust crates | Implement ourselves | pulldown-latex stalled, others unusable |
| Parser boundary | Rust: parse + layout; Swift: render only | Maximize reuse; thin render layer |
| Where layout runs | Rust (not Swift/Kotlin) | Font metrics are static; no platform API |
| FFI format | C ABI + DisplayList as data | Portable, serializable, debuggable |
| Font metrics | Start with embedded Latin Modern Math | Reuse KaTeX-validated data |
| Validation | Pixel golden test vs KaTeX | Only quantifiable correctness check |

---

## 10. Summary

**Value proposition:**

- **One implementation, three platforms:** Rust does parse + layout; iOS / Android / Web need ~200 LOC each for rendering.
- **Evidence-based:** Full reference in KaTeX and validation at each layer.
- **Controlled risk:** Golden tests give quantified quality; AI speeds up mechanical work.
- **Reasonable effort:** ~32 person-days solo, ~20 with two people in parallel.

> Goal: `\frac{-b \pm \sqrt{b^2-4ac}}{2a}` looks pixel-identical on iOS, Android, and Web.
