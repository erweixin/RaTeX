# RaTeX Binding Architecture

## Overview / 概述

RaTeX exposes a single Rust core (`ratex-ffi`) as a C ABI static/shared library.
Each platform wraps this library with a thin native layer:

- **iOS** — Swift Package + CoreGraphics renderer
- **Android** — JNI + Kotlin + Android Canvas renderer
- **Flutter** — Dart FFI + CustomPainter renderer

RaTeX 通过 `ratex-ffi` crate 对外暴露一个 C ABI 静态/动态库。
每个平台只需实现一层薄薄的 native wrapper：

- **iOS** — Swift Package + CoreGraphics 渲染
- **Android** — JNI + Kotlin + Android Canvas 渲染
- **Flutter** — Dart FFI + CustomPainter 渲染

---

## Data flow / 数据流

```
LaTeX string  (UTF-8)
        │
        ▼
  ratex_parse_and_layout()        ← C ABI  (crates/ratex-ffi)
        │
        ▼  JSON string (UTF-8)
  [platform JSON decode]
        │
        ▼  DisplayList struct
  [platform native renderer]
        │
        ▼
  Screen  (UIView / Canvas / CustomPaint)
```

---

## C ABI  (`crates/ratex-ffi`)

### Exported functions

| Signature | Description |
|-----------|-------------|
| `const char* ratex_parse_and_layout(const char* latex)` | Parse + layout → JSON DisplayList. Caller must free with `ratex_free_display_list`. Returns NULL on error. |
| `void ratex_free_display_list(char* json)` | Free the JSON string. NULL is a no-op. |
| `const char* ratex_get_last_error(void)` | Thread-local last error. Valid until next call on this thread. Do NOT free. |

### Build artifacts

| Target | Artifact | Used by |
|--------|----------|---------|
| `aarch64-apple-ios` | `libratex_ffi.a` | iOS device |
| `aarch64-apple-ios-sim` | `libratex_ffi.a` | iOS simulator (Apple Silicon) |
| `x86_64-apple-ios` | `libratex_ffi.a` | iOS simulator (Intel) |
| `aarch64-linux-android` | `libratex_ffi.so` | Android arm64-v8a |
| `armv7-linux-androideabi` | `libratex_ffi.so` | Android armeabi-v7a |
| `x86_64-linux-android` | `libratex_ffi.so` | Android x86_64 |

---

## DisplayList JSON format / DisplayList JSON 格式

```jsonc
{
  "width":  5.02,   // total width in em units (乘以 fontSize 得到屏幕像素)
  "height": 1.84,   // ascent above baseline (基线以上高度)
  "depth":  0.21,   // descent below baseline (基线以下深度)
  "items": [
    // ---- GlyphPath: a glyph rendered as outline path commands ----
    {
      "GlyphPath": {
        "x": 0.0, "y": 0.0,   // position (top-left of glyph, in em)
        "scale": 1.0,           // uniform scale applied to commands
        "font": "KaTeX_Main-Regular",
        "char_code": 120,       // Unicode code point
        "commands": [
          { "MoveTo": { "x": 0.1, "y": 0.7 } },
          { "CubicTo": { "x1": 0.2, "y1": 0.5, "x2": 0.4, "y2": 0.3, "x": 0.6, "y": 0.1 } },
          { "Close": null }
        ],
        "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 }
      }
    },
    // ---- Line: horizontal rule (fraction bar, etc.) ----
    { "Line": { "x": 0.1, "y": 0.9, "width": 4.8, "thickness": 0.04,
                "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 } } },
    // ---- Rect: filled rectangle ----
    { "Rect": { "x": 0.5, "y": 1.0, "width": 2.0, "height": 0.5,
                "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 } } },
    // ---- Path: arbitrary outline (radical, delimiter) ----
    { "Path": { "x": 0.0, "y": 0.0, "commands": [ /* ... */ ],
                "fill": true,
                "color": { "r": 0.0, "g": 0.0, "b": 0.0, "a": 1.0 } } }
  ]
}
```

### Coordinate system / 坐标系

- Origin: top-left corner of bounding box（原点在包围盒左上角）
- X increases rightward（X 向右增大）
- Y increases downward（Y 向下增大）
- Baseline at Y = `height × fontSize`（基线在 `height × fontSize` 处）
- All values in **em units**; multiply by `fontSize` (pt/px) for screen coords（所有坐标单位为 em，乘以 `fontSize` 得屏幕坐标）

### PathCommand variants / 路径指令

| Variant | Fields | Meaning |
|---------|--------|---------|
| `MoveTo` | `x, y` | Move pen to (x, y) |
| `LineTo` | `x, y` | Draw line to (x, y) |
| `CubicTo` | `x1, y1, x2, y2, x, y` | Cubic Bézier to (x, y) with control points (x1,y1) and (x2,y2) |
| `QuadTo` | `x1, y1, x, y` | Quadratic Bézier to (x, y) with control point (x1,y1) |
| `Close` | — | Close current subpath |

---

## Platform-specific notes / 各平台注意事项

### iOS

- Library is linked **statically** (`libratex_ffi.a`) via XCFramework.
- Swift uses `RaTeXEngine.shared.parse(_:)` which calls the C ABI and decodes JSON.
- `RaTeXRenderer` draws into a `CGContext` using `CGMutablePath` for glyph outlines.
- All rendering happens on a background task; UI updates on `MainActor`.

### Android

- Library is loaded **dynamically** (`libratex_ffi.so`) via `System.loadLibrary`.
- A Rust JNI bridge (`crates/ratex-ffi/src/jni.rs`) converts between JNI types and the C ABI.
- `RaTeXRenderer` draws onto `android.graphics.Canvas` using `android.graphics.Path`.
- `RaTeXEngine.parse` is a `suspend` function; runs on `Dispatchers.Default`.

### Flutter

- iOS: library is loaded via `DynamicLibrary.process()` (statically linked).
- Android: library is loaded via `DynamicLibrary.open('libratex_ffi.so')`.
- `RaTeXPainter` is a `CustomPainter` that draws on `dart:ui Canvas`.
- For heavy formulas, run `parseAndLayout` in an isolate via `compute()`.

---

## Build overview / 构建概览

```
# iOS (produces RaTeX.xcframework)
bash platforms/ios/build-ios.sh

# Android (produces jniLibs/*.so)
bash platforms/android/build-android.sh

# Verify Rust
cargo build --release -p ratex-ffi
cargo test -p ratex-ffi
```

---

## Adding a new platform / 新增平台

1. **Depend on `ratex-ffi`**: the library provides everything needed.
2. **Load the library**: static link (iOS, macOS, Linux) or `dlopen` (Android, Windows).
3. **Bind 3 functions**: `ratex_parse_and_layout`, `ratex_free_display_list`, `ratex_get_last_error`.
4. **Decode JSON**: map the `DisplayList` schema to native types.
5. **Render**: loop over `items` and dispatch on `GlyphPath / Line / Rect / Path`.
