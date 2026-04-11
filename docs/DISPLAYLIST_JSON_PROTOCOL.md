# DisplayList JSON Protocol (FFI / WASM)

RaTeX exposes its renderable output as a **DisplayList** serialized to JSON. This JSON is consumed by:

- **C ABI / FFI** (`ratex-ffi`): `ratex_parse_and_layout(...) -> char*` (UTF-8 JSON)
- **WASM** (`ratex-wasm`): `renderLatex(latex) -> string` (JSON)

This document defines the **schema**, **compatibility rules**, and **change policy** for that JSON.

---

## 1. Scope and stability

- **Units**: all geometry values are in **em**. Multiply by the caller’s `fontSize` (px/pt) to get screen units.
- **Coordinate system**:
  - origin at the **top-left** of the DisplayList bounding box
  - \(x\) increases to the right, \(y\) increases downward
  - baseline is at \(y = height\) (in em)
- **Stability target**:
  - Prior to 1.0, the JSON format may evolve.
  - At 1.0, the JSON format becomes a **public protocol**: breaking changes require a major-version bump (or an explicit, versioned payload; see below).

---

## 2. Compatibility rules (decoders)

If you implement a decoder in another language, follow these rules:

- **Ignore unknown fields** at any level (forward compatibility).
- **Treat missing optional fields as defaults** (backward compatibility).
- **Do not assume field ordering**.
- **Numeric values are finite**: RaTeX clamps NaN/Infinity to `0` before serialization (FFI/WASM).

### Variant tagging

Enums use **serde internally-tagged** format:

- `DisplayItem` objects carry `"type"` alongside their fields.
- `PathCommand` objects carry `"type"` alongside their fields.

---

## 3. Schema (current)

### 3.1 Top-level `DisplayList`

```jsonc
{
  "version": 1,
  "width":  5.02,
  "height": 1.84,
  "depth":  0.21,
  "items": [ /* DisplayItem[] */ ]
}
```

- **version**: protocol version (integer). Decoders may treat missing `version` as `0`.
- **width**: total width (em)
- **height**: ascent above baseline (em)
- **depth**: descent below baseline (em)
- **items**: ordered drawing commands

### 3.2 `DisplayItem`

#### GlyphPath

```jsonc
{
  "type": "GlyphPath",
  "x": 0.0,
  "y": 0.0,
  "scale": 1.0,
  "font": "Main-Regular",
  "char_code": 120,
  "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 }
}
```

- **font**: short KaTeX-style font ID (e.g. `Main-Regular`, `Math-Italic`, `Size1-Regular`). Platform renderers map this to the actual font resource.
- **char_code**: Unicode code point.
- **commands**: **not currently serialized** for `GlyphPath` (intentionally omitted to reduce payload). Decoders must not require it.

#### Line

```jsonc
{
  "type": "Line",
  "x": 0.1,
  "y": 0.9,
  "width": 4.8,
  "thickness": 0.04,
  "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 },
  "dashed": false
}
```

- **dashed**: optional; defaults to `false` when absent.

#### Rect

```jsonc
{
  "type": "Rect",
  "x": 0.5,
  "y": 1.0,
  "width": 2.0,
  "height": 0.5,
  "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 }
}
```

#### Path

```jsonc
{
  "type": "Path",
  "x": 0.0,
  "y": 0.0,
  "commands": [ /* PathCommand[] */ ],
  "fill": true,
  "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 }
}
```

### 3.3 `PathCommand`

```jsonc
{ "type": "MoveTo",  "x": 0.1, "y": 0.7 }
{ "type": "LineTo",  "x": 0.2, "y": 0.8 }
{ "type": "CubicTo", "x1": 0.2, "y1": 0.5, "x2": 0.4, "y2": 0.3, "x": 0.6, "y": 0.1 }
{ "type": "QuadTo",  "x1": 0.2, "y1": 0.5, "x": 0.6, "y": 0.1 }
{ "type": "Close" }
```

---

## 4. Change policy (1.0+)

### Allowed (non-breaking)

- Add new **optional** fields (decoders ignore unknown fields; treat missing as defaults).
- Add new `DisplayItem` or `PathCommand` variants **only if** decoders are written to ignore unknown variants (recommended). If a decoder hard-fails on unknown `"type"`, this becomes breaking for that decoder.

### Breaking changes (require major bump or explicit versioning)

- Renaming/removing fields
- Changing semantics of existing fields
- Changing enum tagging format
- Changing coordinate system or units

### Optional versioning in the payload

The payload includes a top-level `version` field (currently `1`). Decoders should treat a missing
field as `0` and remain tolerant to future versions by ignoring unknown fields/variants.

