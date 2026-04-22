/**
 * <ratex-formula> Web Component — drop-in, works with any framework or plain HTML.
 *
 * Usage:
 *   1. Load fonts (once): <link rel="stylesheet" href="node_modules/ratex-wasm/fonts.css" />
 *   2. Register component: <script type="module" src="node_modules/ratex-wasm/dist/ratex-formula.js"></script>
 *   3. Use: <ratex-formula latex="\frac{-b \pm \sqrt{b^2-4ac}}{2a}"></ratex-formula>
 *
 * If fonts.css is not imported, the component will attempt auto-injection
 * (resolves fonts.css relative to import.meta.url within the same package).
 */

import { initRatex, renderLatexToDisplayList, renderToCanvas } from "./index.js";
import type { WebRenderOptions } from "./renderer.js";

const TAG = "ratex-formula";

/** Default em / padding matching the demo's drawDisplayList, to prevent incorrect canvas sizes like 32×32 */
const DEFAULT_EM = 48;
const DEFAULT_PAD = 16;

function ensureFontsLoaded(): void {
  const id = "ratex-wasm-fonts";
  if (document.getElementById(id)) return;
  try {
    const href = new URL("../fonts.css", import.meta.url).href;
    const link = document.createElement("link");
    link.id = id;
    link.rel = "stylesheet";
    link.href = href;
    document.head.appendChild(link);
  } catch {
    console.warn("[ratex-formula] Could not auto-load fonts.css. Include <link rel=\"stylesheet\" href=\"ratex-wasm/fonts.css\"> for math glyphs.");
  }
}

export class RatexFormulaElement extends HTMLElement {
  static get observedAttributes(): string[] {
    return ["latex", "font-size", "padding", "background-color", "color"];
  }

  private _canvas: HTMLCanvasElement | null = null;

  connectedCallback(): void {
    ensureFontsLoaded();
    if (!this._canvas) {
      const root = this.attachShadow({ mode: "open" });
      const canvas = document.createElement("canvas");
      this._canvas = canvas;
      root.appendChild(canvas);
    }
    // Pre-warm WASM as early as possible so it's ready by the time _renderWhenReady needs it.
    initRatex().catch(() => {});
    this._renderWhenReady();
  }

  disconnectedCallback(): void {}

  attributeChangedCallback(_name: string, _oldValue: string | null, _newValue: string | null): void {
    this._renderWhenReady();
  }

  get latex(): string {
    return this.getAttribute("latex") ?? "";
  }

  set latex(value: string) {
    if (value != null) this.setAttribute("latex", value);
    else this.removeAttribute("latex");
  }

  private _getOptions(): Partial<WebRenderOptions> {
    const fontSize = this.getAttribute("font-size");
    const padding = this.getAttribute("padding");
    const bg = this.getAttribute("background-color");
    const opts: Partial<WebRenderOptions> = {};
    // Use safe defaults matching the demo's drawDisplayList to avoid a 32×32 blank canvas when em/pad are 0
    const em = fontSize != null ? Number(fontSize) : DEFAULT_EM;
    const pad = padding != null ? Number(padding) : DEFAULT_PAD;
    opts.fontSize = Number.isFinite(em) && em > 0 ? em : DEFAULT_EM;
    opts.padding = Number.isFinite(pad) && pad >= 0 ? pad : DEFAULT_PAD;
    if (bg != null) opts.backgroundColor = bg;
    return opts;
  }

  /**
   * Canvas size calculation matching drawDisplayList in demo/index.html:
   * totalH = height + depth, w = ceil(width*em + 2*pad), h = ceil(totalH*em + 2*pad)
   */
  private _setCanvasSizeFromDisplayList(
    widthEm: number,
    heightEm: number,
    depthEm: number,
    em: number,
    pad: number
  ): void {
    if (!this._canvas) return;
    const totalH = heightEm + depthEm;
    const w = Math.ceil(widthEm * em + 2 * pad);
    const h = Math.ceil(totalH * em + 2 * pad);
    this._canvas.width = Math.max(1, w);
    this._canvas.height = Math.max(1, h);
  }

  private async _renderWhenReady(): Promise<void> {
    if (!this._canvas || !this.isConnected) return;
    const latex = this.latex.trim();
    if (!latex) {
      this._canvas.width = 0;
      this._canvas.height = 0;
      return;
    }
    try {
      await initRatex();
      const opts = this._getOptions();
      const color = this.getAttribute("color") ?? undefined;
      const em = opts.fontSize ?? DEFAULT_EM;
      const pad = opts.padding ?? DEFAULT_PAD;
      // Matching demo drawDisplayList order: get display list first, set canvas size from width/height/depth, then draw
      const displayList = renderLatexToDisplayList(latex, color);
      this._setCanvasSizeFromDisplayList(
        displayList.width,
        displayList.height,
        displayList.depth,
        em,
        pad
      );
      renderToCanvas(displayList, this._canvas, opts);
    } catch (e) {
      console.error("[ratex-formula] latex=" + JSON.stringify(latex.slice(0, 80)), e);
      const ctx = this._canvas.getContext("2d");
      if (ctx) {
        this._canvas.width = 200;
        this._canvas.height = 24;
        ctx.fillStyle = "#ccc";
        ctx.font = "14px sans-serif";
        ctx.fillText("RaTeX error", 0, 18);
      }
    }
  }
}

if (typeof customElements !== "undefined" && !customElements.get(TAG)) {
  customElements.define(TAG, RatexFormulaElement);
}
