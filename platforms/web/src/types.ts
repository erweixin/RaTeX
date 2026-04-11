/**
 * TypeScript types mirroring RaTeX Rust DisplayList (ratex-types).
 * Used by web-render to draw on Canvas 2D.
 */

export interface Color {
  r: number;
  g: number;
  b: number;
  a: number;
}

export type KnownDisplayItemType = "GlyphPath" | "Line" | "Rect" | "Path";

export type PathCommand =
  | { type: "MoveTo"; x: number; y: number }
  | { type: "LineTo"; x: number; y: number }
  | { type: "CubicTo"; x1: number; y1: number; x2: number; y2: number; x: number; y: number }
  | { type: "QuadTo"; x1: number; y1: number; x: number; y: number }
  | { type: "Close" };

export type DisplayItem =
  | {
      type: "GlyphPath";
      x: number;
      y: number;
      scale: number;
      font: string;
      char_code: number;
      /** Placeholder bounding-box paths; omitted in serialized output since v0.0.11. */
      commands?: PathCommand[];
      color: Color;
    }
  | {
      type: "Line";
      x: number;
      y: number;
      width: number;
      thickness: number;
      color: Color;
      /** Optional; defaults to false when absent. */
      dashed?: boolean;
    }
  | {
      type: "Rect";
      x: number;
      y: number;
      width: number;
      height: number;
      color: Color;
    }
  | {
      type: "Path";
      x: number;
      y: number;
      commands: PathCommand[];
      fill: boolean;
      color: Color;
    }
  | UnknownDisplayItem;

/**
 * Forward-compatibility: allow newer DisplayItem variants.
 * Decoders should ignore unknown item types (protocol requires this).
 */
export type UnknownDisplayItem = {
  type: Exclude<string, KnownDisplayItemType>;
  // Allow extra fields without failing type-checks at call sites.
  [key: string]: unknown;
};

export interface DisplayList {
  /** DisplayList JSON protocol version (optional). Missing implies version 0. */
  version?: number;
  items: DisplayItem[];
  width: number;
  height: number;
  depth: number;
}
