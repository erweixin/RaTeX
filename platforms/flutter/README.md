# RaTeX — Flutter Integration Guide

Native Flutter rendering of LaTeX math formulas via Dart FFI and CustomPainter.
No WebView, no JavaScript.

---

## How it works

```
LaTeX string
    ↓ RaTeXFfi.parseAndLayout()   [Dart FFI → libratex_ffi]
JSON DisplayList
    ↓ DisplayList.fromJson()       [Dart JSON decode]
DisplayList
    ↓ RaTeXPainter.paint()         [flutter/canvas]
CustomPaint Widget
```

---

## Prerequisites

| Tool | Version |
|------|---------|
| Flutter | 3.10+ |
| Dart | 3.0+ |
| Rust | 1.75+ |

Build the native libraries first:
- **iOS**: run `bash platforms/ios/build-ios.sh` (produces `RaTeX.xcframework`)
- **Android**: run `bash platforms/android/build-android.sh` (produces `.so` files)

---

## Add to your Flutter project

In your `pubspec.yaml`:
```yaml
dependencies:
  ratex_flutter:
    path: /path/to/RaTeX/platforms/flutter
```

---

## Usage

### Widget (recommended)

```dart
import 'package:ratex_flutter/ratex.dart';

class MathPage extends StatelessWidget {
  @override
  Widget build(BuildContext context) => Scaffold(
    body: Center(
      child: RaTeXWidget(
        latex: r'\frac{-b \pm \sqrt{b^2-4ac}}{2a}',
        fontSize: 28,
        onError: (e) => debugPrint('RaTeX: $e'),
      ),
    ),
  );
}
```

### Low-level CustomPainter

```dart
import 'package:ratex_flutter/ratex.dart';

final dl      = RaTeXEngine.instance.parseAndLayout(r'\sum_{n=1}^\infty \frac{1}{n^2}');
final painter = RaTeXPainter(displayList: dl, fontSize: 24);

// In a CustomPaint widget:
CustomPaint(painter: painter, size: Size(painter.widthPx, painter.totalHeightPx))
```

### Async (large formulas)

```dart
import 'package:flutter/foundation.dart';

final dl = await compute(
  (latex) => RaTeXEngine.instance.parseAndLayout(latex),
  r'\prod_{n=1}^\infty \left(1 - \frac{1}{n^2}\right)',
);
```

---

## Coordinate system

Same as iOS/Android: all coordinates are in **em units**, multiplied by `fontSize`
(logical pixels) to get screen coordinates. Y increases downward from the top of
the bounding box. The baseline is at Y = `height × fontSize`.

---

## File map

| File | Purpose |
|------|---------|
| `pubspec.yaml` | Flutter plugin manifest |
| `lib/ratex.dart` | Public API: `RaTeXEngine`, `RaTeXWidget` |
| `lib/src/display_list.dart` | Dart JSON types (DisplayList, DisplayItem, …) |
| `lib/src/ratex_ffi.dart` | Dart FFI bindings to `libratex_ffi` |
| `lib/src/ratex_painter.dart` | `CustomPainter` drawing loop |
