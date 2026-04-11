# Roadmap

High-level direction (feature gaps, platforms, performance) should be tracked here as the project grows. This file is intentionally short; detailed design lives in `docs/`.

## Golden tests: unnumbered display math only

`tests/golden/test_cases.txt` uses **only unnumbered** AMS-style environments: `equation*`, `gather*`, `align*`, `alignat*`, etc. The numbered forms (`equation`, `gather`, `align`, …) are **not** used in the golden suite.

**Rationale**

1. **RaTeX** does not yet implement automatic equation numbering and tag placement comparable to LaTeX/KaTeX for these environments.
2. **KaTeX** reference screenshots for numbered multiline environments can show **tags overlapping ink**; that is a poor, unstable baseline for raster comparison.

**After editing `test_cases.txt`**, regenerate references and RaTeX outputs so indices stay aligned:

- KaTeX PNGs: `node tools/golden_compare/generate_reference.mjs` (see script header).
- RaTeX PNGs: `scripts/update_golden_output.sh`.

---

## Golden 测试：仅无编号 display 环境

`tests/golden/test_cases.txt` 中 **一律** 使用带星号的环境（`equation*`、`gather*`、`align*`、`alignat*` 等），**不使用**会产生自动编号的 `equation`、`gather`、`align` 等写法。

**原因**：(1) RaTeX 1.0 目标为 KaTeX 兼容渲染，当前**不实现自动公式编号**，编号/标签依赖显式 `\tag`；(2) KaTeX 在多行编号场景下参考图易出现编号与公式重叠，不适合作为稳定的像素对比基准。

修改 `test_cases.txt` 后请重跑 `generate_reference.mjs` 与 `update_golden_output.sh`，以同步 `fixtures/` 与 `output/`。
