# Roadmap

High-level direction (feature gaps, platforms, performance) should be tracked here as the project grows. This file is intentionally short; detailed design lives in `docs/`.

## Golden tests: mostly unnumbered display math

`tests/golden/test_cases.txt` is **mostly** starred AMS environments (`equation*`, `gather*`, `align*`, `alignat*`, …) so KaTeX reference PNGs stay a **stable** baseline for ink-based comparison.

A **small** set of lines exercises **automatic numbering** (`equation`, `align`, `gather`, `alignat`), `\tag`, and `\nonumber` / `\notag` (see near the end of the file). RaTeX implements these; KaTeX reference shots for multiline numbered rows can still show **tags overlapping ink**, which is a weaker baseline for strict pixel diffs—use those lines mainly for regression coverage.

**After editing `test_cases.txt`**, keep indices aligned:

- KaTeX PNGs: `node tools/golden_compare/generate_reference.mjs` (see script header).
- RaTeX PNGs: `scripts/update_golden_output.sh`.

---

## Golden 测试：以无编号环境为主

`tests/golden/test_cases.txt` **主体**仍为带星号的环境（`equation*`、`gather*`、`align*`、`alignat*` 等），以便 KaTeX 对照图作为**稳定**的像素/墨量基准。

文件**末尾附近**保留少量用例，覆盖 **自动编号**（`equation`、`align`、`gather`、`alignat`）以及 `\tag`、`\nonumber` / `\notag`。RaTeX 已支持这些特性；KaTeX 在多行编号场景下参考图仍可能出现**编号与笔迹重叠**，不适合作为严苛像素对比的权威基准，以功能回归为主。

修改 `test_cases.txt` 后请重跑 `generate_reference.mjs` 与 `update_golden_output.sh`，以同步 `fixtures/` 与 `output/`。
