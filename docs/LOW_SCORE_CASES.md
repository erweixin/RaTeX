# Golden test cases with score below 0.5

Cases to fix or explicitly accept as “different by design”. Sorted by score ascending (worst first).

**Total: 29 cases** (as of **2026-03-19**)

---

## How the score is computed

Script: `tools/golden_compare/compare_golden.py`. For each case it crops both PNGs to ink, normalizes height, then combines:

`score = 0.4 × IoU + 0.2 × recall + 0.2 × aspect_sim + 0.2 × width_sim`

- **IoU / recall**: overlap of “ink” pixels vs KaTeX reference (`INK_THRESHOLD = 240` on RGB).
- **aspect_sim / width_sim**: geometry of the cropped content and normalized width.

Default **pass threshold** for the script summary is **0.30** (see `--threshold`); this document lists cases with **score below 0.5** as “visually fragile” even if they pass CI.

---

## Themes (current batch)

| Theme | Case IDs (PNG stem) | Notes |
|-------|---------------------|--------|
| `\url` / `\href` | 0831, 0359 | Web/hyperlink semantics; raster baseline may not be comparable. |
| `\imageof`, `\origof` | 0375, 0599 | Symbol/glyph or metric mismatch vs KaTeX. |
| `\overbrace` / `\underbrace` + `\text` | 0603, 0810 | Brace + label layout; see also `docs/KATEX_SVG_PATH_PLAN.md`. |
| `\cancel`, `\bcancel`, `\xcancel` | 0074, 0146, 0890 | Strikeout drawing vs KaTeX. |
| `\copyright`, `\textregistered`, `\char"263a` | 0198, 0770, 0157 | Text/compound symbol rendering. |
| Arrays / environments | 0061, 0286, 0315, 0037, 0880, 0878, 0879, 0730 | `arraystretch`, `equation`, `gather`, `alignat`, starred matrices, `smallmatrix`. |
| Delimiters / `\genfrac` | 0454, 0695, 0662, 0092, 0321 | `\llbracket`, `\rrbracket`, `\rBrace`, `\Biggm\vert`, `\genfrac`. |
| Other | 0699 (`\rule`), 0156 (`\cfrac`), 0263 (`\dotsi`) | Spacing, nested fractions, dots. |

Stretchy-arrow SVG work is still documented in [`KATEX_SVG_PATH_PLAN.md`](./KATEX_SVG_PATH_PLAN.md); **this** list is driven by whatever scores below 0.5 **today**, not only arrows.

---

## Checklist

After fixing a case, re-render and re-compare:

```bash
./scripts/update_golden_output.sh
python3 tools/golden_compare/compare_golden.py --threshold 0.30
```

Then mark the row below with `[x]`.

---

| # | test_case | score | formula | fixed |
|---|-----------|-------|---------|-------|
| 1 | 0831 | 0.044 | `\url{https://katex.org/}` | [ ] |
| 2 | 0359 | 0.138 | `\href{https://katex.org/}{\KaTeX}` | [ ] |
| 3 | 0375 | 0.163 | `\imageof` | [ ] |
| 4 | 0599 | 0.163 | `\origof` | [ ] |
| 5 | 0810 | 0.221 | `\underbrace{x+⋯+x}_{n\text{ times}}` | [ ] |
| 6 | 0603 | 0.235 | `\overbrace{x+⋯+x}^{n\text{ times}}` | [ ] |
| 7 | 0074 | 0.367 | `\bcancel{5}` | [ ] |
| 8 | 0146 | 0.374 | `\cancel{5}` | [ ] |
| 9 | 0198 | 0.375 | `\copyright` | [ ] |
| 10 | 0770 | 0.375 | `\text{\textregistered}` | [ ] |
| 11 | 0699 | 0.378 | `x\rule[6pt]{2ex}{1ex}x` | [ ] |
| 12 | 0061 | 0.387 | `\def\arraystretch{1.5} \begin{array}{cc} a & b \\ c & d \end{array}` | [ ] |
| 13 | 0157 | 0.390 | `\char"263a` | [ ] |
| 14 | 0880 | 0.392 | `\begin{vmatrix*}[r] 0 & -1 \\ -1 & 0 \end{vmatrix*}` | [ ] |
| 15 | 0286 | 0.397 | `\begin{equation} a = b + c \end{equation}` | [ ] |
| 16 | 0878 | 0.405 | `\begin{Vmatrix*}[r] 0 & -1 \\ -1 & 0 \end{Vmatrix*}` | [ ] |
| 17 | 0890 | 0.405 | `\xcancel{ABC}` | [ ] |
| 18 | 0730 | 0.414 | `\begin{smallmatrix} a & b \\ c & d \end{smallmatrix}` | [ ] |
| 19 | 0662 | 0.418 | `\rBrace` | [ ] |
| 20 | 0454 | 0.432 | `\llbracket` | [ ] |
| 21 | 0138 | 0.434 | `{n\brace k}` | [ ] |
| 22 | 0695 | 0.441 | `\rrbracket` | [ ] |
| 23 | 0315 | 0.467 | `\begin{gather} a=b \\ e=b+c \end{gather}` | [ ] |
| 24 | 0037 | 0.468 | `\begin{alignat}{2} 10&x+ &3&y = 2 \\ 3&x+&13&y = 4 \end{alignat}` | [ ] |
| 25 | 0092 | 0.473 | `\Biggm\vert` | [ ] |
| 26 | 0156 | 0.476 | `\cfrac{2}{1+\cfrac{2}{1+\cfrac{2}{1}}}` | [ ] |
| 27 | 0263 | 0.479 | `\int_{A_1}\int_{A_2}\dotsi` | [ ] |
| 28 | 0879 | 0.487 | `\begin{vmatrix} a & b \\ c & d \end{vmatrix}` | [ ] |
| 29 | 0321 | 0.497 | `\genfrac ( ] {2pt}{0}a{a+1}` | [ ] |

---

## SOP: Fixing low-scoring **stretchy arrow** cases

Applies to `\xrightarrow`, `\xleftarrow`, `\xtwoheadrightarrow`, `\xtwoheadleftarrow`, `\xmapsto`, and similar commands. For **braces**, prefer the brace path plan in [`KATEX_SVG_PATH_PLAN.md`](./KATEX_SVG_PATH_PLAN.md).

### 1. Find KaTeX SVG path data

Install KaTeX next to the lexer tool (fonts already come from here in `update_golden_output.sh`):

```bash
cd tools/lexer_compare && npm install
```

Then inspect paths, for example:

```bash
grep -A2 "xmapsto\|leftmapsto\|rightarrow" \
  tools/lexer_compare/node_modules/katex/dist/katex.min.js | head -20
```

(Use `katex.js` instead of `katex.min.js` if you prefer the non-minified bundle.)

Key ideas:

- Single-headed arrows (`rightarrow`, `leftarrow`): viewBox `400000 × 534`, center line `vb_cy = 261`
- Double-headed arrows (`twoheadrightarrow`, `twoheadleftarrow`): viewBox `400000 × 334`, center line `vb_cy = 167`
- Compound commands (e.g. `xmapsto`) use formats like `[["leftmapsto", "rightarrow"], min_width, vb_height]`

### 2. Coordinate transform rules

Uniform scale: `s = height_em / vb_height`

```
x_new = x_vb * s + x_shift
y_new = (y_vb - vb_cy) * s
```

**x_shift rules**:

- Right-pointing arrow (tip at right): `x_shift = width_em - 400000 * s`
- Left-pointing arrow (tip at left, x≈0): `x_shift = 0.0`

### 3. Implementation (in `katex_stretchy_arrow_path` in `crates/ratex-layout/src/katex_svg.rs`)

1. Locate the path string in KaTeX’s bundle and add it as a `const`
2. Parse with `parse_svg_path()`
3. Apply coordinate transform with `scale_cmd_twohead_uniform(cmd, s, vb_cy, x_shift)`
4. Clip to render region with `clip_path_to_rect(&cmds, 0.0, width_em, y_min, y_max)`
5. For compound commands (e.g. `\xmapsto`), process each path segment separately then merge (`combined.extend(...)`)

### 4. Common pitfalls

| Issue | Cause | Fix |
|-------|-------|-----|
| Left arrow head disappears | Negative `x_shift` places head outside clip region | Use `x_shift = 0.0` for left arrows; the head is already near x≈0 in the viewBox |
| Shaft segments have color gaps / path not closed | `clip_path_to_rect` skips the clip boundary start point when a gap is detected | When detecting a gap, emit `LineTo(a)` before `LineTo(b)` |
| Closing edge missing | Loop uses `0..n-1`, missing the last edge from p[n-1] back to p[0] | Change to `for i in 0..n`, use `contour[(i+1) % n]` for the next point |
| Shaft too thin to be visible | `arrow_h` is too small; shaft thickness `40 * s` &lt; 1px | Increase `arrow_h` or compensate with stroke width |
| Path reflection logic wrong | Extra x-reflection applied to left-pointing arrows | Use left-pointing path constants directly (e.g. `LEFTARROW`), no reflection needed |

### 5. Verification

PNG stem `NNNN` is the **1-based case index** (same integer as the filename). The matching formula is **line `NNNN`** in `tests/golden/test_cases.txt` (first line = case `0001`).

```bash
# Render a single case (example: stem 0603 → line 603)
sed -n '603p' tests/golden/test_cases.txt | \
  cargo run --release -p ratex-render --bin render -- \
  --font-dir tools/lexer_compare/node_modules/katex/dist/fonts \
  --output-dir /tmp/ratex_test/

./scripts/update_golden_output.sh
python3 tools/golden_compare/compare_golden.py --threshold 0.30
```

---

## Regenerate this list

After layout or golden pipeline changes, refresh numbers from the same inputs as CI:

```bash
./scripts/update_golden_output.sh   # optional if output/ already fresh
python3 tools/golden_compare/compare_golden.py --verbose
```

Recompute every row with **score below 0.5** (e.g. small Python snippet importing `tools/golden_compare/compare_golden.py`, same loops as `main()`), then update this file: **table**, **Total**, and **as of** date.
