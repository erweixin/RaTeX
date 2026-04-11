/**
 * Web-render: draw RaTeX DisplayList to Canvas 2D.
 * Coordinates are in em units; we scale by font size (em) and add padding.
 */

import type { Color, DisplayItem, DisplayList, PathCommand } from "./types.js";

export interface WebRenderOptions {
  /** Font size in pixels (1em). Default 40. */
  fontSize?: number;
  /** Padding in pixels. Default 10. */
  padding?: number;
  /** Fill style for background. Default "white". */
  backgroundColor?: string;
  /** CSS font-family for math glyphs (GlyphPath). Must load a math font in your page. Default uses KaTeX_Main when available. */
  mathFontFamily?: string;
}

const DEFAULT_OPTIONS: Required<Omit<WebRenderOptions, "mathFontFamily">> & { mathFontFamily: string } = {
  fontSize: 40,
  padding: 10,
  backgroundColor: "white",
  mathFontFamily: 'KaTeX_Main, "Latin Modern Math", "Cambria Math", serif',
};

/**
 * Map a RaTeX FontId string (e.g. "Math-Italic") to a complete CSS font declaration
 * that selects the correct KaTeX @font-face variant (family + style + weight).
 */
function fontIdToCss(fontId: string, sizePx: number): string {
  switch (fontId) {
    case "AMS-Regular":        return `${sizePx}px KaTeX_AMS`;
    case "Caligraphic-Regular":return `${sizePx}px KaTeX_Caligraphic`;
    case "Fraktur-Regular":    return `${sizePx}px KaTeX_Fraktur`;
    case "Fraktur-Bold":       return `bold ${sizePx}px KaTeX_Fraktur`;
    case "Main-Bold":          return `bold ${sizePx}px KaTeX_Main`;
    case "Main-BoldItalic":    return `italic bold ${sizePx}px KaTeX_Main`;
    case "Main-Italic":        return `italic ${sizePx}px KaTeX_Main`;
    case "Main-Regular":       return `${sizePx}px KaTeX_Main`;
    case "Math-BoldItalic":    return `italic bold ${sizePx}px KaTeX_Math`;
    case "Math-Italic":        return `italic ${sizePx}px KaTeX_Math`;
    case "SansSerif-Bold":     return `bold ${sizePx}px KaTeX_SansSerif`;
    case "SansSerif-Italic":   return `italic ${sizePx}px KaTeX_SansSerif`;
    case "SansSerif-Regular":  return `${sizePx}px KaTeX_SansSerif`;
    case "Script-Regular":     return `${sizePx}px KaTeX_Script`;
    case "Size1-Regular":      return `${sizePx}px KaTeX_Size1`;
    case "Size2-Regular":      return `${sizePx}px KaTeX_Size2`;
    case "Size3-Regular":      return `${sizePx}px KaTeX_Size3`;
    case "Size4-Regular":      return `${sizePx}px KaTeX_Size4`;
    case "Typewriter-Regular": return `${sizePx}px KaTeX_Typewriter`;
    default:                   return `${sizePx}px KaTeX_Main`;
  }
}

function colorToCss(c: Color): string {
  const r = Math.round(c.r * 255);
  const g = Math.round(c.g * 255);
  const b = Math.round(c.b * 255);
  if (c.a >= 1 - 1e-5) return `rgb(${r},${g},${b})`;
  return `rgba(${r},${g},${b},${c.a})`;
}

function applyPathCommands(ctx: CanvasRenderingContext2D, commands: PathCommand[], em: number, ox: number, oy: number): void {
  for (const cmd of commands) {
    switch (cmd.type) {
      case "MoveTo":
        ctx.moveTo(ox + cmd.x * em, oy + cmd.y * em);
        break;
      case "LineTo":
        ctx.lineTo(ox + cmd.x * em, oy + cmd.y * em);
        break;
      case "CubicTo":
        ctx.bezierCurveTo(
          ox + cmd.x1 * em, oy + cmd.y1 * em,
          ox + cmd.x2 * em, oy + cmd.y2 * em,
          ox + cmd.x * em, oy + cmd.y * em
        );
        break;
      case "QuadTo":
        ctx.quadraticCurveTo(
          ox + cmd.x1 * em, oy + cmd.y1 * em,
          ox + cmd.x * em, oy + cmd.y * em
        );
        break;
      case "Close":
        ctx.closePath();
        break;
    }
  }
}

/**
 * Draw a single DisplayItem onto the canvas context.
 * Assumes ctx is already scaled/translated so that (0,0) is content top-left and 1 unit = 1em.
 * For GlyphPath: layout only provides placeholder rect paths; we draw the actual character
 * with fillText using mathFontFamily so letters/numbers render correctly.
 */
function drawItem(
  ctx: CanvasRenderingContext2D,
  item: DisplayItem,
  em: number,
  mathFontFamily: string
): void {
  switch (item.type) {
    case "GlyphPath": {
      const g = item as Extract<DisplayItem, { type: "GlyphPath" }>;
      // Layout emits placeholder rectangle paths; browser has no font outline data.
      // Use item.font (FontId string from Rust) to select the exact KaTeX variant.
      // No save/restore needed: fillText uses absolute coords and doesn't modify the CTM.
      ctx.font = fontIdToCss(g.font, g.scale * em);
      ctx.textBaseline = "alphabetic";
      ctx.textAlign = "left";
      ctx.fillStyle = colorToCss(g.color);
      ctx.fillText(String.fromCodePoint(g.char_code), g.x * em, g.y * em);
      break;
    }
    case "Line": {
      const l = item as Extract<DisplayItem, { type: "Line" }>;
      const x = l.x * em;
      const y = l.y * em;
      const w = l.width * em;
      const t = Math.max(0.5, l.thickness * em);
      const css = colorToCss(l.color);
      if (l.dashed) {
        ctx.save();
        ctx.beginPath();
        ctx.strokeStyle = css;
        ctx.lineWidth = t;
        ctx.lineCap = "butt";
        // Simple, visually stable dash pattern in pixel units.
        ctx.setLineDash([t * 3, t * 3]);
        ctx.moveTo(x, y);
        ctx.lineTo(x + w, y);
        ctx.stroke();
        ctx.restore();
      } else {
        ctx.fillStyle = css;
        ctx.fillRect(x, y - t / 2, w, t);
      }
      break;
    }
    case "Rect": {
      const r = item as Extract<DisplayItem, { type: "Rect" }>;
      ctx.fillStyle = colorToCss(r.color);
      ctx.fillRect(r.x * em, r.y * em, r.width * em, r.height * em);
      break;
    }
    case "Path": {
      const p = item as Extract<DisplayItem, { type: "Path" }>;
      ctx.beginPath();
      applyPathCommands(ctx, p.commands, em, p.x * em, p.y * em);
      ctx.strokeStyle = colorToCss(p.color);
      ctx.fillStyle = colorToCss(p.color);
      if (p.fill) ctx.fill();
      else ctx.stroke();
      break;
    }
    default: {
      // Forward compatibility: ignore items newer than this renderer.
      void mathFontFamily;
      break;
    }
  }
}

/**
 * Render a DisplayList to the given canvas.
 * Resizes the canvas to fit the content (list width/height/depth in em) plus padding.
 */
export function renderToCanvas(
  displayList: DisplayList,
  canvas: HTMLCanvasElement,
  options: WebRenderOptions = {}
): void {
  const opts = { ...DEFAULT_OPTIONS, ...options };
  const em = opts.fontSize;
  const pad = opts.padding;

  const totalH = displayList.height + displayList.depth;
  const pixelW = Math.ceil(displayList.width * em + 2 * pad);
  const pixelH = Math.ceil(totalH * em + 2 * pad);

  canvas.width = Math.max(1, pixelW);
  canvas.height = Math.max(1, pixelH);

  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Could not get 2d context");

  ctx.fillStyle = opts.backgroundColor;
  ctx.fillRect(0, 0, canvas.width, canvas.height);

  ctx.save();
  ctx.translate(pad, pad);
  const fontFamily = opts.mathFontFamily ?? DEFAULT_OPTIONS.mathFontFamily;
  for (const item of displayList.items) {
    drawItem(ctx, item, em, fontFamily);
  }
  ctx.restore();
}
