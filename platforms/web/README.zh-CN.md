# RaTeX Web（WASM + Web-Render）

在浏览器中使用 RaTeX：Rust 编译为 WASM 负责解析与排版；TypeScript **web-render** 根据 DisplayList 在 Canvas 2D 上绘制。

> **说明：** Web 端建议直接使用 [KaTeX](https://katex.org/) — 它是浏览器数学渲染上成熟、更优的选择。本 WASM 方案**并非 Web 生产环境的最优解**，主要用于**跨平台对比与测试**（与 iOS/Android 同一 RaTeX 引擎，仅渲染路径不同）。

## 架构

- **ratex-wasm**（`crates/ratex-wasm`）：Rust → WASM，同时导出向后兼容的 `renderLatex(latex, color?, displayMode?)` 和便于扩展的 `renderLatexWithOptions(latex, options?)`，均返回 DisplayList JSON。`displayMode` 默认为 `true`；传 `false` 使用行内/文本样式。
- **web-render**（`src/renderer.ts`）：将 DisplayList 绘制到 Canvas 2D。`GlyphPath` 条目通过 Canvas `fillText` 与 `char_code` 加载的 KaTeX 字体绘制；页面需加载数学字体（随包附带的 `fonts.css` 已覆盖此需求）。
- **入口**（`src/index.ts`）：初始化 WASM，同时提供带类型的 options 对象重载和原有位置参数 API。

## 开箱即用

无需构建，直接使用已发布的 npm 包：

1. **安装** — `npm install ratex-wasm`（或 `yarn add ratex-wasm`）。
2. **在页面中** — 引入字体并注册 Web 组件，然后用自定义标签：
   ```html
   <link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
   <script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>
   <ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" color="#1E88E5"></ratex-formula>
   <ratex-formula latex="\frac{1}{2}" display-mode="false"></ratex-formula>
   ```
3. 支持属性：`latex`、`font-size`、`padding`、`background-color`、`color`、`display-mode`（默认 `"true"`，传 `"false"` 使用行内样式）；也可通过 JS 设置 `element.latex = '...'` 和 `element.displayMode = false`。

## 构建

**环境要求**：需安装 [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) 与 Rust 工具链，否则 `npm run build` 会报错。

```bash
# 安装 wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
cd platforms/web
npm install   # 安装 katex 等 devDependency 用于字体复制
npm run build # copy-fonts → build:wasm（生成 pkg/）→ build:ts
```

输出：`pkg/`（WASM）与 `fonts/`（KaTeX woff2/woff，由 `fonts.css` 引用）。

## 使用

### 即用 Web 组件：`<ratex-formula>`

无需打包器，适用于任意框架或纯 HTML。

```html
<!-- 1. 字体（一次即可；组件也会尝试自动注入） -->
<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />

<!-- 2. 注册自定义元素 -->
<script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>

<!-- 3. 使用 -->
<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" padding="16" color="#1E88E5"></ratex-formula>
<ratex-formula latex="x^2 + y^2 = z^2" display-mode="false"></ratex-formula>
```

支持的属性：`latex`、`font-size`、`padding`、`background-color`、`color`、`display-mode`（默认 `"true"`；传 `"false"` 使用行内/文本样式）。也可通过 JS 设置 `element.latex = '...'` 和 `element.displayMode = false`。

**React**：直接使用 DOM 标签；React 18+ 能正确渲染自定义元素。传入字符串时建议用 `ref` 设置 `el.latex = '...'`（优于 `dangerouslySetInnerHTML`）。

**Vue**：Vue 3 默认将非 Vue 标签视为自定义元素。若需显式配置：`app.config.compilerOptions.isCustomElement = (tag) => tag === 'ratex-formula'`（Vue 3.2+ 可选）。

### 方式一：TypeScript/ESM（编程 API）

```ts
import { initRatex, renderLatexToCanvas } from './index.js';

await initRatex();
const canvas = document.querySelector('canvas');
renderLatexToCanvas('\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}', canvas, {
  fontSize: 48,
  padding: 16,
  backgroundColor: 'white',
}, '#1E88E5');
```

**说明**：页面必须加载数学字体，否则字母和数字会显示为方框。可使用 KaTeX 的 CSS（见仓库根目录 `demo/`）或提供 Latin Modern Math。本包**自带 KaTeX 字体**（无需 CDN）：使用 `<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />` 或 `import 'ratex-wasm/fonts.css';`，字体从包内加载。

### 方式二：仅获取 DisplayList JSON

```ts
import { initRatex, renderLatex, renderLatexWithOptions } from './index.js';

await initRatex();
const json = renderLatex('x^2 + y^2 = z^2', '#1E88E5');
const displayList = JSON.parse(json);

// 推荐：便于向前扩展的 options 对象，并支持精确的归一化 RGBA。
const inlineJson = renderLatexWithOptions('\\frac{1}{2}', {
  displayMode: false,
  color: { r: 0.12, g: 0.53, b: 0.9, a: 0.75 },
});

// npm 包入口也支持等价的 renderLatex(latex, options) 重载。

// 原有位置参数调用保持兼容。
const legacyInlineJson = renderLatex('\\frac{1}{2}', undefined, false);
```
