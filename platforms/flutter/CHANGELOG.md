## 0.0.8

- Publish to pub.dev with bundled Android jniLibs and iOS RaTeX.xcframework (out-of-the-box)

## 0.0.3

- Add KaTeX font bundling for glyph rendering via `ParagraphBuilder`
- GlyphPath items now render with correct KaTeX fonts instead of placeholder paths
- Fix line thickness minimum (0.5px) for thin rules
- Support iOS xcframework and Flutter plugin registration

## 0.0.2

- Initial iOS FFI binding via `DynamicLibrary.process()`
- DisplayList-based rendering pipeline

## 0.0.1

- Initial release
- Dart FFI bindings to libratex_ffi
- RaTeXWidget and RaTeXPainter
