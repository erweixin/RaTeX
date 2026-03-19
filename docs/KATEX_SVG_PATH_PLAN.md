# Plan: Use KaTeX's Exact SVG Paths for All Stretchy Elements

## Context

Many **stretchy** math elements still differ from KaTeX because of SVG path data; the **current** set of worst golden scores (not only arrows) is tracked in [`LOW_SCORE_CASES.md`](./LOW_SCORE_CASES.md). Historically, a large share of low scores were arrow/brace path issues. Currently:
- `katex_stretchy_arrow_path()` only handles 5 arrow types (rightarrow, leftarrow, twoheadright, twoheadleft, xmapsto)
- All other stretchy arrows fall back to `stretchy_accent_path()` in engine.rs, which draws **custom simplified arrows** that look very different from KaTeX
- `horiz_brace_path()` uses custom quadratic curves instead of KaTeX's actual brace paths
- `\vec` uses a custom filled arrow instead of KaTeX's glyph-based path

**Solution**: Import ALL SVG path data from KaTeX's `path` object and use KaTeX's `katexImagesData` table to generalize the rendering for all stretchy elements.

## Files to Modify

1. **`crates/ratex-layout/src/katex_svg.rs`** — Main changes (add paths, add lookup table, generalize function)
2. **`crates/ratex-layout/src/engine.rs`** — Update callers to use generalized function

## Step 1: Add All Missing KaTeX Path Constants

Add these path constants to `katex_svg.rs` (copy directly from KaTeX's `path` object in `katex.js:744-822`):

| Path Name | Used By |
|-----------|---------|
| `doubleleftarrow` | `\xLeftarrow`, `\xLeftrightarrow` |
| `doublerightarrow` | `\xRightarrow`, `\Overrightarrow`, `\xLeftrightarrow` |
| `leftharpoon` | `\xleftharpoonup`, `\overleftharpoon` |
| `leftharpoondown` | `\xleftharpoondown` |
| `leftharpoonplus` | `\xleftrightharpoons` |
| `leftharpoondownplus` | `\xrightleftharpoons` |
| `rightharpoon` | `\xrightharpoonup`, `\overrightharpoon` |
| `rightharpoondown` | `\xrightharpoondown` |
| `rightharpoonplus` | `\xrightleftharpoons` |
| `rightharpoondownplus` | `\xleftrightharpoons` |
| `lefthook` | `\xhookrightarrow` |
| `righthook` | `\xhookleftarrow` |
| `leftbrace` | `\overbrace` |
| `midbrace` | `\overbrace` |
| `rightbrace` | `\overbrace` |
| `leftbraceunder` | `\underbrace` |
| `midbraceunder` | `\underbrace` |
| `rightbraceunder` | `\underbrace` |
| `leftToFrom` | `\xtofrom`, `\xrightleftarrows` |
| `rightToFrom` | `\xtofrom` |
| `baraboveleftarrow` | `\xrightleftarrows` |
| `rightarrowabovebar` | `\xrightleftarrows` |
| `longequal` | `\xlongequal` |
| `leftlinesegment` | `\overlinesegment`, `\underlinesegment` |
| `rightlinesegment` | `\overlinesegment`, `\underlinesegment` |
| `vec` (KaTeX's actual path) | `\vec` |

## Step 2: Create KaTeX Images Data Lookup Table

Add a struct and lookup function mirroring KaTeX's `katexImagesData`:

```rust
struct KatexImageData {
    paths: &'static [&'static str],     // 1, 2, or 3 path names
    min_width: f64,                       // minimum width in em
    vb_height: f64,                       // viewBox height (÷1000 = em height)
    align: Option<&'static str>,          // "xMinYMin" or "xMaxYMin" (only for single-path)
}

fn katex_image_data(label: &str) -> Option<KatexImageData>
```

Key entries (from `katex.js:6875-6921`):
- Single-path: `xrightarrow` → `["rightarrow"], 1.469, 522, "xMaxYMin"`
- Two-path: `xleftrightarrow` → `["leftarrow", "rightarrow"], 1.75, 522`
- Three-path: `overbrace` → `["leftbrace", "midbrace", "rightbrace"], 1.6, 548`

## Step 3: Generalize the Rendering Function

Replace `katex_stretchy_arrow_path()` with a general function:

```rust
pub fn katex_stretchy_path(label: &str, width_em: f64) -> Option<Vec<PathCommand>>
```

Logic:
1. Look up label in `katex_image_data()`
2. Compute `height_em = vb_height / 1000.0` and `s = 1.0 / 1000.0` (since KaTeX scale is 1000:1)
3. Based on number of paths:
   - **1 path**: Current logic — scale with `s`, apply x_shift based on alignment, clip to `[0, width_em]`
   - **2 paths**: Generalize from current `xmapsto` logic:
     - Left path: x_shift = 0, scale, clip to `[0, width_em]`
     - Right path: x_shift = `width_em - 400000*s`, scale, clip to `[0, width_em]`
     - Combine both
   - **3 paths** (braces): Same pattern:
     - Left path: x_shift = 0
     - Center path: x_shift = `width_em/2 - 200000*s`
     - Right path: x_shift = `width_em - 400000*s`
     - Combine all three

The y-transform remains: `y_new = (y_vb - vb_cy) * s` where `vb_cy = vb_height / 2.0`

## Step 4: Update Callers in engine.rs

1. **`stretchy_accent_path()`** (~line 2420): Replace the large match block with a call to `katex_stretchy_path()`. Keep the simple fallback arrow only as a last resort.

2. **`layout_xarrow()`** (~line 2258): Use `katex_stretchy_path()` instead of `katex_stretchy_arrow_path()`.

3. **`layout_horiz_brace()`** / `horiz_brace_path()`: Use `katex_stretchy_path("overbrace"/"underbrace", width)` instead of custom quadratic curves.

4. **`layout_accent()` for `\vec`**: Use KaTeX's actual vec path from the path table.

5. **Accent paths** (`\overrightarrow`, `\underleftarrow`, etc.): These are also in `katexImagesData`, so the generalized function handles them too.

## Step 5: Fix ViewBox Height Values

Update existing entries to match KaTeX's `katexImagesData`:
- rightarrow/leftarrow: 534 → **522**
- LEFTMAPSTO (in xmapsto): 534 → **522** (same viewBox as rightarrow)

## Affected Low-Score Cases

This change should fix or significantly improve these categories:

| Category | Cases | Count |
|----------|-------|-------|
| Stretchy arrows (hook, harpoon, double, etc.) | #4-#12, #25-#31, #36 | ~15 |
| Overbrace/underbrace | #13, #39, #43 | 3 |
| Over/under arrows (accent form) | #18, #19, #22 | 3 |
| Bar accent | #38 | 1 |

## Verification

```bash
# Build
cargo build --release -p ratex-render

# Test individual cases
sed -n '357p' tests/golden/test_cases.txt | \
  cargo run --release -p ratex-render --bin render -- \
  --font-dir tools/lexer_compare/node_modules/katex/dist/fonts \
  --output-dir /tmp/ratex_test/

# Run full comparison
./scripts/update_golden_output.sh
python3 tools/golden_compare/compare_golden.py --threshold 0.30
```

## Implementation Order

1. Add all path constants (mechanical, low risk)
2. Add lookup table (mechanical)
3. Generalize rendering function (core logic, test incrementally)
4. Update engine.rs callers one at a time, verifying each
5. Fix viewBox heights
