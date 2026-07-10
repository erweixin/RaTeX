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

export interface RenderColorRgba {
  r: number;
  g: number;
  b: number;
  a: number;
}

export type RenderColor = string | RenderColorRgba;

export interface RenderLatexOptions {
  /** `true` (default) for display/block style; `false` for inline/text style. */
  displayMode?: boolean;
  /** Default formula color as a supported color string or normalized RGBA components. */
  color?: RenderColor;
}

export interface RatexWasmCapabilities {
  /** The positional `renderLatex(latex, color, displayMode)` API is supported. */
  displayMode?: boolean;
}

export interface RatexWasmModule {
  renderLatex: (latex: string, color?: string, displayMode?: boolean) => string;
  renderLatexWithOptions?: (latex: string, options?: RenderLatexOptions) => string;
  /** Explicit capabilities for injected modules that do not expose an options API. */
  capabilities?: RatexWasmCapabilities;
}

interface InitializedRatexWasmModule {
  renderLatex: RatexWasmModule["renderLatex"];
  renderLatexWithOptions?: RatexWasmModule["renderLatexWithOptions"];
  supportsPositionalDisplayMode: boolean;
}

let wasmModule: InitializedRatexWasmModule | null = null;
let _initPromise: Promise<void> | null = null;

/**
 * Initialize the WASM module. Safe to call concurrently — subsequent calls share
 * the same in-flight promise so WASM is loaded at most once.
 * Pass the URL to the WASM package's init (e.g. from your bundler or script tag).
 * Injected positional-only modules should declare `capabilities.displayMode`;
 * support is never inferred from `Function.length`.
 */
export function initRatex(init?: () => Promise<RatexWasmModule>): Promise<void> {
  if (wasmModule) return Promise.resolve();
  if (_initPromise) return _initPromise;
  _initPromise = _doInit(init);
  return _initPromise;
}

async function _doInit(init?: () => Promise<RatexWasmModule>): Promise<void> {
  if (init) {
    wasmModule = normalizeWasmModule(await init());
    return;
  }
  // Default: dynamic import of the wasm-pack generated pkg
  const pkg = await import("../pkg/ratex_wasm.js");
  const wasmPackage = pkg as unknown as RatexWasmModule & {
    default?: () => Promise<unknown>;
  };
  if (typeof wasmPackage.default !== "function") {
    throw new Error("ratex_wasm default export should be an init function");
  }
  await wasmPackage.default(); // init WASM (sets internal wasm); do not use its return value
  // Use the pkg's JS wrapper renderLatex (which reads string from memory), not raw wasm.renderLatex (which returns [ptr, len])
  wasmModule = normalizeWasmModule(wasmPackage);
}

function normalizeWasmModule(module: RatexWasmModule): InitializedRatexWasmModule {
  if (typeof module.renderLatex !== "function") {
    throw new Error("RaTeX WASM module should export a renderLatex function");
  }
  const renderLatexWithOptions = typeof module.renderLatexWithOptions === "function"
    ? module.renderLatexWithOptions
    : undefined;
  return {
    renderLatex: module.renderLatex,
    renderLatexWithOptions,
    // The options export is detected directly at its call site. Positional-only
    // injected modules must opt in because Function.length is changed by
    // default parameters, bind(), transpilers, and wrappers.
    supportsPositionalDisplayMode: module.capabilities?.displayMode === true,
  };
}

/**
 * Parse LaTeX and return the display list as a JSON string (or throw on parse error).
 * `displayMode` defaults to `true`; pass `false` for inline/text style.
 * Requires initRatex() to have been called first.
 */
export function renderLatex(latex: string, options?: RenderLatexOptions): string;
export function renderLatex(latex: string, color?: string, displayMode?: boolean): string;
export function renderLatex(
  latex: string,
  colorOrOptions?: string | RenderLatexOptions,
  displayMode = true
): string {
  if (colorOrOptions != null && typeof colorOrOptions === "object") {
    return renderLatexWithOptions(latex, colorOrOptions);
  }
  if (!wasmModule) throw new Error("RaTeX WASM not initialized. Call initRatex() first.");
  if (displayMode === false) {
    if (wasmModule.renderLatexWithOptions) {
      return wasmModule.renderLatexWithOptions(latex, {
        color: colorOrOptions,
        displayMode,
      });
    }
    if (!wasmModule.supportsPositionalDisplayMode) {
      throw unsupportedDisplayModeError();
    }
  }
  return wasmModule.renderLatex(latex, colorOrOptions, displayMode);
}

/**
 * Parse LaTeX using a forward-compatible options object.
 * Existing positional renderLatex() calls remain supported.
 */
export function renderLatexWithOptions(latex: string, options: RenderLatexOptions = {}): string {
  if (!wasmModule) throw new Error("RaTeX WASM not initialized. Call initRatex() first.");
  const resolvedOptions = options ?? {};
  if (wasmModule.renderLatexWithOptions) {
    return wasmModule.renderLatexWithOptions(latex, resolvedOptions);
  }

  // Compatibility for callers that inject an older WASM module. Structured
  // RGBA falls back to 8-bit hex because the legacy API only accepts strings.
  // Published legacy modules also lack the positional displayMode argument,
  // so reject inline mode instead of silently returning display-style output.
  if (resolvedOptions.displayMode === false && !wasmModule.supportsPositionalDisplayMode) {
    throw unsupportedDisplayModeError();
  }
  const color = typeof resolvedOptions.color === "string"
    ? resolvedOptions.color
    : resolvedOptions.color == null
      ? undefined
      : rgbaToHex(resolvedOptions.color);
  return wasmModule.renderLatex(latex, color, resolvedOptions.displayMode ?? true);
}

function unsupportedDisplayModeError(): Error {
  return new Error(
    "The initialized RaTeX WASM module does not support displayMode. Upgrade the WASM module or use displayMode: true."
  );
}

function rgbaToHex(color: RenderColorRgba): string {
  const components = [color.r, color.g, color.b, color.a];
  const names = ["r", "g", "b", "a"];
  return `#${components.map((component, index) => {
    if (!Number.isFinite(component) || component < 0 || component > 1) {
      throw new Error(`invalid options.color.${names[index]}: expected a finite number in [0, 1], got ${component}`);
    }
    return Math.round(component * 255).toString(16).padStart(2, "0");
  }).join("")}`;
}

/**
 * Parse LaTeX and return the display list as a DisplayList object.
 * Throws if LaTeX is invalid or WASM not initialized.
 */
export function renderLatexToDisplayList(latex: string, options?: RenderLatexOptions): DisplayList;
export function renderLatexToDisplayList(latex: string, color?: string, displayMode?: boolean): DisplayList;
export function renderLatexToDisplayList(
  latex: string,
  colorOrOptions?: string | RenderLatexOptions,
  displayMode = true
): DisplayList {
  const json = colorOrOptions != null && typeof colorOrOptions === "object"
    ? renderLatexWithOptions(latex, colorOrOptions)
    : renderLatex(latex, colorOrOptions, displayMode);
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
  colorOrLayoutOptions?: string | RenderLatexOptions,
  displayMode = true
): DisplayList {
  const displayList = colorOrLayoutOptions != null && typeof colorOrLayoutOptions === "object"
    ? renderLatexToDisplayList(latex, colorOrLayoutOptions)
    : renderLatexToDisplayList(latex, colorOrLayoutOptions, displayMode);
  renderToCanvas(displayList, canvas, options);
  return displayList;
}

export { renderToCanvas } from "./renderer.js";
export type { DisplayList, DisplayItem, Color, PathCommand } from "./types.js";
export type { WebRenderOptions } from "./renderer.js";
