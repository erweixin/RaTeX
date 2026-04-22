# RaTeX Web (WASM + Web-Render)

Use RaTeX in the browser: Rust compiled to WASM handles parsing and layout; TypeScript **web-render** draws on Canvas 2D from the DisplayList.

> **Note:** For web projects we recommend using [KaTeX](https://katex.org/) directly — it is the mature, optimized choice for browser math rendering. This WASM build is **not intended as the best solution** for production web; it exists mainly for **cross-platform comparison and testing** (same RaTeX engine as iOS/Android, different render path).

## Architecture

- **ratex-wasm** (`crates/ratex-wasm`): Rust → WASM, exports `renderLatex(latex: string, color?: string) => string` returning DisplayList JSON.
- **web-render** (`src/renderer.ts`): Renders the DisplayList to Canvas 2D. `GlyphPath` items are drawn via Canvas `fillText` using `char_code` and the loaded KaTeX font; the page must load a math font (bundled `fonts.css` covers this).
- **Entry** (`src/index.ts`): Initializes WASM and provides `renderLatexToCanvas(latex, canvas, options, color?)` for one-step rendering.

## Out of the box

No build required — use the published npm package:

1. **Install** — `npm install ratex-wasm` (or `yarn add ratex-wasm`).
2. **Use** — see [Drop-in Web Component](#drop-in-web-component-ratex-formula) below.

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
<ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}" font-size="48" padding="16" color="#1E88E5"></ratex-formula>
<ratex-formula latex="x^2 + y^2 = z^2"></ratex-formula>
```

Supported attributes: `latex`, `font-size`, `padding`, `background-color`, `color`. You can also set `element.latex = '...'` via JS.

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
}, '#1E88E5');
```

**Note:** The page must load a math font, or letters/numbers will show as boxes. You can use KaTeX’s CSS (see repo-root `demo/`) or provide Latin Modern Math. This package **bundles KaTeX fonts** (no CDN): use `<link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />` or `import 'ratex-wasm/fonts.css';` and fonts load from the package.

### Option 2: DisplayList JSON only

```ts
await initRatex();
const json = renderLatex('x^2 + y^2 = z^2', '#1E88E5');
const displayList = JSON.parse(json);
```
