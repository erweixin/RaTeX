# RaTeX

**简体中文** | [English](README.md)

**纯 Rust 实现的 KaTeX 兼容数学渲染引擎 — 无 JavaScript、无 WebView、无 DOM。**

一个 Rust 核心，一套显示列表，各平台原生渲染。

```
\frac{-b \pm \sqrt{b^2-4ac}}{2a}   →   iOS · Android · Flutter · React Native · Web · PNG · SVG
```

**[→ 在线演示](https://erweixin.github.io/RaTeX/demo/live.html)** — 输入 LaTeX，对比 RaTeX vs KaTeX ·
**[→ 支持表](https://erweixin.github.io/RaTeX/demo/support-table.html)** — 全量测试公式的 RaTeX vs KaTeX 对比

---

## 为什么选 RaTeX？

目前主流的跨平台数学渲染方案都依赖浏览器或 JavaScript 引擎跑 LaTeX，带来隐藏 WebView 占用 50–150 MB 内存、首屏公式要等 JS 启动、无法保证离线等问题。

RaTeX 完全去掉 Web 栈：

| | KaTeX (Web) | MathJax | **RaTeX** |
|---|---|---|---|
| 运行时 | V8 + DOM | V8 + DOM | **纯 Rust** |
| 移动端 | WebView | WebView | **原生** |
| 离线 | 视情况 | 视情况 | **支持** |
| 包体积 | ~280 kB JS | ~500 kB JS | **0 kB JS** |
| 内存模型 | GC / 堆 | GC / 堆 | **可预期** |
| 语法覆盖 | 100% | ~100% | **~99%** |

---

## 能渲染什么

**数学公式** — ~99% 的 KaTeX 语法：分数、根号、积分、矩阵、各类环境、伸缩定界符等。

**化学方程式** — 通过 `\ce` 和 `\pu` 完整支持 mhchem：

```latex
\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}
\ce{Fe^{2+} + 2e- -> Fe}
\pu{1.5e-3 mol//L}
```

**物理单位** — `\pu` 支持符合 IUPAC 规范的数值+单位表达式。

---

## 平台支持

| 平台 | 方式 | 状态 |
|---|---|---|
| **iOS** | XCFramework + Swift / CoreGraphics | 开箱即用 |
| **Android** | JNI + Kotlin + Canvas · AAR | 开箱即用 |
| **Flutter** | Dart FFI + `CustomPainter` | 开箱即用 |
| **React Native** | C ABI Native 模块 · iOS/Android 原生视图 | 开箱即用 |
| **Web** | WASM → Canvas 2D · `<ratex-formula>` Web 组件 | 可用 |
| **服务端 / CI** | tiny-skia → PNG 光栅化 | 可用 |
| **SVG** | `ratex-svg` → 自包含矢量 SVG 导出 | 可用 |

---

## 架构

```mermaid
flowchart LR
    A["LaTeX 字符串\n(数学 · \\ce · \\pu)"]
    subgraph core["Rust 核心"]
        B[ratex-lexer]
        C[ratex-parser\nmhchem \\ce / \\pu]
        D[ratex-layout]
        E[DisplayList]
    end
    F[ratex-ffi\niOS · Android · Flutter · RN]
    G[ratex-wasm\nWeb / Canvas 2D]
    H[ratex-render\nPNG · tiny-skia]
    I[ratex-svg\nSVG]
    A --> B --> C --> D --> E
    E --> F
    E --> G
    E --> H
    E --> I
```

| Crate | 职责 |
|---|---|
| `ratex-types` | 共享类型：`DisplayItem`、`DisplayList`、`Color`、`MathStyle` |
| `ratex-font` | 兼容 KaTeX 的字体度量与符号表 |
| `ratex-lexer` | LaTeX → token 流 |
| `ratex-parser` | token 流 → ParseNode AST；含 mhchem `\ce` / `\pu` |
| `ratex-layout` | AST → LayoutBox 树 → DisplayList |
| `ratex-ffi` | C ABI：向各原生平台暴露完整流水线 |
| `ratex-wasm` | WASM：流水线 → DisplayList JSON（浏览器） |
| `ratex-render` | 服务端：DisplayList → PNG（tiny-skia） |
| `ratex-svg` | SVG 导出：DisplayList → SVG 字符串 |

---

## 快速开始

**环境要求：** Rust 1.70+（[rustup](https://rustup.rs)）

```bash
git clone https://github.com/erweixin/RaTeX.git
cd RaTeX
cargo build --release
```

### 渲染为 PNG

```bash
echo '\frac{1}{2} + \sqrt{x}' | cargo run --release -p ratex-render

echo '\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}' | cargo run --release -p ratex-render
```

### 渲染为 SVG

```bash
# 默认模式：字形输出为 <text> 元素（正确显示需要 KaTeX 网络字体）
echo '\frac{1}{2} + \sqrt{x}' | cargo run --release -p ratex-svg --features cli

# 自包含模式：将字形轮廓嵌入为 <path>，无需外部字体
echo '\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}' | \
  cargo run --release -p ratex-svg --features cli -- \
  --font-dir /path/to/katex/fonts --output-dir ./out
```

`standalone` feature（即 `cli` 所依赖的特性）会从 KaTeX TTF 文件中提取字形轮廓并内嵌到 SVG，生成无需任何 CSS 或网络字体即可正确渲染的完全自包含文件。

### 在浏览器中使用（WASM）

```bash
npm install ratex-wasm
```

```html
<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
<script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>

<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48"></ratex-formula>
<ratex-formula latex="\ce{CO2 + H2O <=> H2CO3}" font-size="32"></ratex-formula>
```

完整说明见 [`platforms/web/README.md`](platforms/web/README.md)。

### 各平台胶水层

| 平台 | 文档 |
|---|---|
| iOS | [`platforms/ios/README.md`](platforms/ios/README.md) |
| Android | [`platforms/android/README.md`](platforms/android/README.md) |
| Flutter | [`platforms/flutter/README.md`](platforms/flutter/README.md) |
| React Native | [`platforms/react-native/README.md`](platforms/react-native/README.md) |
| Web | [`platforms/web/README.md`](platforms/web/README.md) |

### 运行测试

```bash
cargo test --all
```

---

## 致谢

RaTeX 深受 [KaTeX](https://katex.org/) 启发——其解析器架构、符号表、字体度量与排版语义是本引擎的基础。化学符号（`\ce`、`\pu`）由 [mhchem](https://mhchem.github.io/MathJax-mhchem/) 状态机的 Rust 移植实现。

---

## 参与贡献

见 [`CONTRIBUTING.md`](CONTRIBUTING.md)。安全问题报告见 [`SECURITY.md`](SECURITY.md)。

---

## 许可证

MIT — Copyright (c) erweixin.
