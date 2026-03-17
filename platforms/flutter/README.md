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

## Installation

### From pub.dev (recommended)

Add to your `pubspec.yaml`:

```yaml
dependencies:
  ratex_flutter: ^0.0.3
```

Then run `flutter pub get`. No native build required — the published package includes prebuilt Android `.so` and iOS `RaTeX.xcframework`.

### From local path (development)

If you use the package from the RaTeX repo:

```yaml
dependencies:
  ratex_flutter:
    path: /path/to/RaTeX/platforms/flutter
```

You must build the native libraries first:

| Platform | Command |
|----------|---------|
| iOS | `bash platforms/ios/build-ios.sh` (produces `RaTeX.xcframework`) |
| Android | `bash platforms/android/build-android.sh` (produces `.so` files) |

**Prerequisites for building from source:** Flutter 3.10+, Dart 3.0+, Rust 1.75+.

---

## Usage

### Widget (recommended)

```dart
import 'package:ratex_flutter/ratex_flutter.dart';

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
import 'package:ratex_flutter/ratex_flutter.dart';

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
| `ios/` | iOS plugin (podspec + RaTeXPlugin.swift); links RaTeX.xcframework |
| `android/` | Android plugin (RaTeXPlugin.kt); uses in-package `jniLibs` for `libratex_ffi.so` |
| `lib/ratex_flutter.dart` | Public API: `RaTeXEngine`, `RaTeXWidget` |
| `lib/src/display_list.dart` | Dart JSON types (DisplayList, DisplayItem, …) |
| `lib/src/ratex_ffi.dart` | Dart FFI bindings to `libratex_ffi` |
| `lib/src/ratex_painter.dart` | `CustomPainter` drawing loop |

---

## Publishing to pub.dev (maintainers)

To publish an **out-of-the-box** package that works without building native code:

1. **Android** — Build and copy JNI libs into the package:
   ```bash
   # From repo root
   ./platforms/android/build-android.sh
   cp -R platforms/android/src/main/jniLibs/* platforms/flutter/android/src/main/jniLibs/
   ```

2. **iOS** — Ensure `RaTeX.xcframework` is inside the package (not a symlink):
   ```bash
   # From repo root
   ./platforms/ios/build-ios.sh
   # If platforms/flutter/ios/RaTeX.xcframework is a symlink, replace with real copy:
   rm -rf platforms/flutter/ios/RaTeX.xcframework
   cp -R platforms/ios/RaTeX.xcframework platforms/flutter/ios/
   ```

3. **Validate and publish**:
   ```bash
   cd platforms/flutter
   dart pub publish --dry-run
   dart pub publish
   ```

   **CI**: Pushing a version tag (e.g. `v0.0.4`) runs [release-flutter.yml](https://github.com/erweixin/RaTeX/blob/main/.github/workflows/release-flutter.yml): it builds Android and iOS native libs, injects them into this package, and runs `dart pub publish`. Ensure the tag matches the `version` in `pubspec.yaml`. Repository secret required: `PUB_DEV_TOKEN` (create at https://pub.dev/settings/tokens).
