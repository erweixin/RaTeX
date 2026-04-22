/**
 * RaTeX for the browser: WASM (parse + layout) + web-render (Canvas 2D).
 *
 * Usage:
 *   import { initRatex, renderLatexToCanvas } from './index.js';
 *   await initRatex();  // load WASM once
 *   renderLatexToCanvas('\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}', canvasElement);
 */

import { renderToCanvas } from "./renderer.js";
import type { DisplayList } from "./types.js";
import type { WebRenderOptions } from "./renderer.js";

let wasmModule: {
  renderLatex: (latex: string, color?: string) => string;
} | null = null;
let _initPromise: Promise<void> | null = null;

/**
 * Initialize the WASM module. Safe to call concurrently — subsequent calls share
 * the same in-flight promise so WASM is loaded at most once.
 * Pass the URL to the WASM package's init (e.g. from your bundler or script tag).
 */
export function initRatex(init?: () => Promise<{ renderLatex: (s: string) => string }>): Promise<void> {
  if (wasmModule) return Promise.resolve();
  if (_initPromise) return _initPromise;
  _initPromise = _doInit(init);
  return _initPromise;
}

async function _doInit(init?: () => Promise<{ renderLatex: (s: string) => string }>): Promise<void> {
  if (init) {
    const module = await init();
    wasmModule = { renderLatex: module.renderLatex };
    return;
  }
  // Default: dynamic import of the wasm-pack generated pkg
  const pkg = await import("../pkg/ratex_wasm.js");
  if (typeof pkg.default !== "function") throw new Error("ratex_wasm default export should be an init function");
  await pkg.default(); // init WASM (sets internal wasm); do not use its return value
  // Use the pkg's JS wrapper renderLatex (which reads string from memory), not raw wasm.renderLatex (which returns [ptr, len])
  wasmModule = { renderLatex: pkg.renderLatex };
}

/**
 * Parse LaTeX and return the display list as a JSON string (or throw on parse error).
 * Requires initRatex() to have been called first.
 */
export function renderLatex(latex: string, color?: string): string {
  if (!wasmModule) throw new Error("RaTeX WASM not initialized. Call initRatex() first.");
  return wasmModule.renderLatex(latex, color);
}

/**
 * Parse LaTeX and return the display list as a DisplayList object.
 * Throws if LaTeX is invalid or WASM not initialized.
 */
export function renderLatexToDisplayList(latex: string, color?: string): DisplayList {
  const json = renderLatex(latex, color);
  try {
    return JSON.parse(json) as DisplayList;
  } catch (e) {
    const preview = typeof json === "string" ? json.slice(0, 400) : String(json);
    const at7 = typeof json === "string" && json.length > 7 ? ` (char at 7: "${json.slice(6, 12)}")` : "";
    if (typeof console !== "undefined" && console.warn) {
      console.warn("[ratex] WASM returned non-JSON. Raw string:", preview, at7);
    }
    const msg =
      e instanceof SyntaxError && typeof json === "string"
        ? `RaTeX: invalid JSON from WASM${at7}. First 300 chars: ${preview.slice(0, 300)}`
        : String(e);
    throw new Error(msg);
  }
}

/**
 * Parse LaTeX and draw the result on the given canvas (web-render).
 * Resizes the canvas to fit. Optional render options (fontSize, padding, backgroundColor).
 */
export function renderLatexToCanvas(
  latex: string,
  canvas: HTMLCanvasElement,
  options?: WebRenderOptions,
  color?: string
): DisplayList {
  const displayList = renderLatexToDisplayList(latex, color);
  renderToCanvas(displayList, canvas, options);
  return displayList;
}

export { renderToCanvas } from "./renderer.js";
export type { DisplayList, DisplayItem, Color, PathCommand } from "./types.js";
export type { WebRenderOptions } from "./renderer.js";
