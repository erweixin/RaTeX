# mhchem 数据与更新说明

Rust 端的 `\ce` / `\pu` 采用 **表驱动**：状态机与部分正则来自 KaTeX 的 `mhchem.js`，经脚本导出为 JSON，运行时由 `crates/ratex-parser/src/mhchem/` 解析。本文说明各文件职责与升级步骤。

---

## 涉及路径

| 路径 | 作用 |
|------|------|
| `tools/mhchem_reference.js` | 与 KaTeX `contrib/mhchem/mhchem.js` 对齐的参考源码（头注释写明 mhchem 版本，如 3.3.0）。**不要手改 JSON 而不更新此文件**，除非刻意做分叉。 |
| `tools/generate_mhchem_data.mjs` | 在 Node 中用 `vm` 加载参考文件，序列化 `stateMachines` 与正则 pattern 源码，写出两个 JSON。 |
| `tools/dump_mhchem_structure.mjs` | 可选：打印各状态机状态数、转移条数，用于对拍或升级后快速 sanity check。 |
| `crates/ratex-parser/src/mhchem/data/machines.json` | 各机器名 → 状态 → `{ pattern, task }` 列表（`task` 含 `nextState`、`revisit`、`toContinue`、`action_`）。 |
| `crates/ratex-parser/src/mhchem/data/patterns_regex.json` | `regex`: pattern 名 → 正则源码字符串；`functionKeys`: 非正则、需在 Rust `patterns.rs` 实现分支的 key 列表。 |
| `crates/ratex-parser/src/mhchem/data.rs` | `include_str!` 嵌入上述 JSON，编译期 `serde` + `fancy-regex` 编译。 |
| `crates/ratex-parser/src/mhchem/json.rs` | `Task` 等结构体；**`action_` 字段需显式 `serde(rename = "action_")`**，避免被 `rename_all = "camelCase"` 误映射。 |

---

## 何时需要重新生成数据

- 从上游 **替换或合并** `tools/mhchem_reference.js`（例如跟 KaTeX 新版本）。
- 确认上游 **`stateMachines` / `patterns` 结构未被脚本假设破坏**（脚本假定存在 `mhchemParser.stateMachines`、`mhchemParser.patterns.patterns`）。

---

## 更新步骤

1. **更新参考实现**  
   用目标版本的 KaTeX `mhchem.js` 覆盖或合入 `tools/mhchem_reference.js`（若上游有 `import katex`，生成脚本会去掉该行并 stub `katex.__defineMacro`）。

2. **生成 JSON**（需 Node）  
   ```bash
   node tools/generate_mhchem_data.mjs
   ```  
   输出：`machines.json`、`patterns_regex.json`。

3. **可选结构检查**  
   ```bash
   node tools/dump_mhchem_structure.mjs
   ```

4. **Rust 侧**  
   - 若 `functionKeys` 有增减，在 `patterns.rs` 中为对应 key 补齐或删除实现。  
   - 若上游新增 `action_` 名称，在 `actions.rs` 中实现并在分发处注册。

5. **测试**  
   ```bash
   cargo test -p ratex-parser
   ```  
   有化学 golden 时，可用 `tests/golden/test_case_ce.txt` 与 `fixtures_ce/`、`output_ce/` 流程做渲染对拍（见 `CONTRIBUTING.md` 与 `docs/PROJECT_STRUCTURE.md` 中 Golden 说明）。

## 与官方手册用例同步

[`tests/golden/test_case_ce.txt`](../tests/golden/test_case_ce.txt) 中的 `\ce` / `\pu` 行可与 [mhchem 在线手册](https://mhchem.github.io/MathJax-mhchem/) 中全部 `::::` / `::::pu` / `::::$` 示例对齐。从上游 `gh-pages` 的 `index.html` 再生成：

```bash
node tools/extract_mhchem_manual_examples.mjs
# 或使用已下载的 HTML：
node tools/extract_mhchem_manual_examples.mjs path/to/index.html
```

脚本会按手册里的规则把示例转成「每行一条」的 LaTeX（与手册渲染为 `\ce{…}` / `\pu{…}` / `$…$` 的方式一致），并做去重。

---

## 与「手写纯 Rust 状态机」的对比（维护角度）

- **优点**：行为与 KaTeX 对齐时，主要 diff 在 `mhchem_reference.js` 与生成结果；引擎层相对稳定。  
- **风险**：JSON 与 Rust 类型字段名不一致会导致静默失败（已通过 `action_` 等约束缓解）；`functionKeys` 必须与 `patterns.rs` 手工列表同步。

---

## 体积说明

`machines.json` 较大时，`include_str!` 会增大 `ratex-parser` 及依赖它的二进制 / wasm 体积。若需优化，可再评估压缩表、`build.rs` 内嵌压缩字节码或 feature 裁剪等方案（超出本文日常更新范围）。
