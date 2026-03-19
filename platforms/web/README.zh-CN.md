# RaTeX Web（WASM + Web-Render）

在浏览器中使用 RaTeX：Rust 编译为 WASM 负责解析与排版；TypeScript **web-render** 根据 DisplayList 在 Canvas 2D 上绘制。

> **说明：** Web 端建议直接使用 [KaTeX](https://katex.org/) — 它是浏览器数学渲染上成熟、更优的选择。本 WASM 方案**并非 Web 生产环境的最优解**，主要用于**跨平台对比与测试**（与 iOS/Android 同一 RaTeX 引擎，仅渲染路径不同）。

## 架构

- **ratex-wasm**（`crates/ratex-wasm`）：Rust → WASM，导出 `renderLatex(latex: string) => string`，返回 DisplayList JSON。
- **web-render**（`src/renderer.ts`）：将 DisplayList 绘制到 Canvas 2D（Line / Rect / Path / GlyphPath）。**GlyphPath** 当前在排版中为占位矩形；浏览器通过 Canvas `fillText` 与数学字体的 `char_code` 绘制字符，因此页面需加载数学字体（如 KaTeX CSS 或 Latin Modern Math）。
- **入口**（`src/index.ts`）：初始化 WASM，提供 `renderLatexToCanvas(latex, canvas, options)` 一步渲染。

## 开箱即用

无需构建，直接使用已发布的 npm 包：

1. **安装** — `npm install ratex-wasm`（或 `yarn add ratex-wasm`）。
2. **在页面中** — 引入字体并注册 Web 组件，然后用自定义标签：
   ```html
   <link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
   <script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>
   <ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48"></ratex-formula>
   ```
3. 支持属性：`latex`、`font-size`、`padding`、`background-color`；也可通过 JS 设置 `element.latex = '...'`。

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
<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" padding="16"></ratex-formula>
<ratex-formula latex="x^2 + y^2 = z^2"></ratex-formula>
```

支持的属性：`latex`、`font-size`、`padding`、`background-color`。也可通过 JS 设置 `element.latex = '...'`。

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
});
```

**说明**：页面必须加载数学字体，否则字母和数字会显示为方框。可使用 KaTeX 的 CSS（见仓库根目录 `demo/`）或提供 Latin Modern Math。本包**自带 KaTeX 字体**（无需 CDN）：使用 `<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />` 或 `import 'ratex-wasm/fonts.css';`，字体从包内加载。

### 方式二：仅获取 DisplayList JSON

```ts
await initRatex();
const json = renderLatex('x^2 + y^2 = z^2');
const displayList = JSON.parse(json);
```

### 方式三：本地 Demo 页

Demo 位于仓库根目录的 `demo/`。构建 web 平台后，在仓库根目录起服务并打开 demo：

```bash
# 在仓库根目录（RaTeX/）
cd platforms/web && npm run build && cd ../..
npx serve .
# 打开 http://localhost:8080/demo/
```

## 与其他平台的关系

- **ratex-ffi**：iOS、Android（JNI）、Flutter、React Native 的 C ABI。
- **ratex-render**：原生 tiny-skia 渲染为 PNG。
- **ratex-wasm + platforms/web**：同一 DisplayList 在浏览器中由 **web-render** 在 Canvas 2D 上绘制。

**web-render** 即“在浏览器中将 DisplayList 绘制到 Canvas 2D”的这一层。
