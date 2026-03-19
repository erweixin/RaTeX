# RaTeX Web (WASM + Web-Render)

Use RaTeX in the browser: Rust compiled to WASM handles parsing and layout; TypeScript **web-render** draws on Canvas 2D from the DisplayList.

> **Note:** For web projects we recommend using [KaTeX](https://katex.org/) directly — it is the mature, optimized choice for browser math rendering. This WASM build is **not intended as the best solution** for production web; it exists mainly for **cross-platform comparison and testing** (same RaTeX engine as iOS/Android, different render path).

## Architecture

- **ratex-wasm** (`crates/ratex-wasm`): Rust → WASM, exports `renderLatex(latex: string) => string` returning DisplayList JSON.
- **web-render** (`src/renderer.ts`): Renders the DisplayList to Canvas 2D (Line / Rect / Path / GlyphPath). **GlyphPath** is currently a placeholder rectangle in layout; the browser draws characters via Canvas `fillText` and a math font using `char_code`, so the page must load a math font (e.g. KaTeX CSS or Latin Modern Math).
- **Entry** (`src/index.ts`): Initializes WASM and provides `renderLatexToCanvas(latex, canvas, options)` for one-step rendering.

## Out of the box

No build required — use the published npm package:

1. **Install** — `npm install ratex-wasm` (or `yarn add ratex-wasm`).
2. **In your page** — Load fonts and register the web component, then use the custom element:
   ```html
   <link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
   <script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>
   <ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48"></ratex-formula>
   ```
3. Supported attributes: `latex`, `font-size`, `padding`, `background-color`; you can also set `element.latex = '...'` via JS.

## Build

**Prerequisites**: [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) (and the Rust toolchain) must be installed. Running `npm run build` without it will error.

```bash
# Install wasm-pack: https://rustwasm.github.io/wasm-pack/installer/
cd platforms/web
npm install   # installs katex devDependency for font copy
npm run build # copy-fonts → build:wasm (generates pkg/) → build:ts
```

Output: `pkg/` (WASM) and `fonts/` (KaTeX woff2/woff, used by `fonts.css`).

## Usage

### Drop-in Web Component: `<ratex-formula>`

No bundler required — works with any framework or plain HTML.

```html
<!-- 1. Fonts (once; the component also attempts auto-injection) -->
<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />

<!-- 2. Register the custom element -->
<script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>

<!-- 3. Use it -->
<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" padding="16"></ratex-formula>
<ratex-formula latex="x^2 + y^2 = z^2"></ratex-formula>
```

Supported attributes: `latex`, `font-size`, `padding`, `background-color`. You can also set `element.latex = '...'` via JS.

**In React**: Use the DOM tag directly; React 18+ renders custom elements correctly. To pass a string, use a `ref` to set `el.latex = '...'` (preferred over `dangerouslySetInnerHTML`).

**In Vue**: Vue 3 treats non-Vue tags as custom elements by default. To configure explicitly: `app.config.compilerOptions.isCustomElement = (tag) => tag === 'ratex-formula'` (optional in Vue 3.2+).

### Option 1: TypeScript/ESM (Programmatic API)

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

**Note:** The page must load a math font, or letters/numbers will show as boxes. You can use KaTeX’s CSS (see repo-root `demo/`) or provide Latin Modern Math. This package **bundles KaTeX fonts** (no CDN): use `<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />` or `import 'ratex-wasm/fonts.css';` and fonts load from the package.

### Option 2: DisplayList JSON only

```ts
await initRatex();
const json = renderLatex('x^2 + y^2 = z^2');
const displayList = JSON.parse(json);
```

### Option 3: Local demo page

Demo page lives in the repo root under `demo/`. After building the web platform, serve the repo root and open the demo:

```bash
# From repo root (RaTeX/)
cd platforms/web && npm run build && cd ../..
npx serve .
# open http://localhost:8080/demo/
```

## Relation to other platforms

- **ratex-ffi**: C ABI for iOS, Android (JNI), Flutter, React Native.
- **ratex-render**: Native tiny-skia rendering to PNG.
- **ratex-wasm + platforms/web**: Same DisplayList in the browser, drawn by **web-render** on Canvas 2D.

So **web-render** is the layer that “draws the DisplayList to Canvas 2D in the browser”.
