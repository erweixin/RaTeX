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

## Out of the box

1. **Add dependency** — add `ratex_flutter: ^0.1.0` to `pubspec.yaml`, then run `flutter pub get`. No native build required — the published package includes prebuilt Android `.so` and iOS XCFramework.
2. **Register fonts** — Flutter does not auto-register plugin fonts for the host app. Copy the [KaTeX font declarations](#font-setup) into your `pubspec.yaml` (see Installation below).
3. **Use** — Use `RaTeXWidget`:
   ```dart
   RaTeXWidget(
     latex: r'\frac{-b \pm \sqrt{b^2-4ac}}{2a}',
     fontSize: 28,
     onError: (e) => debugPrint('RaTeX: $e'),
   )
   ```

---

## Installation

### From pub.dev (recommended)

Add to your `pubspec.yaml`:

```yaml
dependencies:
  ratex_flutter: ^0.1.0
```

Then run `flutter pub get`. No native build required — the published package includes prebuilt Android `.so` and iOS `RaTeX.xcframework`.

#### Font setup

Flutter requires host apps to explicitly declare fonts from plugin packages ([Flutter docs](https://docs.flutter.dev/cookbook/design/package-fonts#from-a-package)). Add the following to the `flutter:` section of your `pubspec.yaml`:

```yaml
flutter:
  fonts:
    - family: KaTeX_AMS
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_AMS-Regular.ttf
    - family: KaTeX_Caligraphic
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Caligraphic-Regular.ttf
        - asset: packages/ratex_flutter/fonts/KaTeX_Caligraphic-Bold.ttf
          weight: 700
    - family: KaTeX_Fraktur
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Fraktur-Regular.ttf
        - asset: packages/ratex_flutter/fonts/KaTeX_Fraktur-Bold.ttf
          weight: 700
    - family: KaTeX_Main
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Main-Regular.ttf
        - asset: packages/ratex_flutter/fonts/KaTeX_Main-Bold.ttf
          weight: 700
        - asset: packages/ratex_flutter/fonts/KaTeX_Main-Italic.ttf
          style: italic
        - asset: packages/ratex_flutter/fonts/KaTeX_Main-BoldItalic.ttf
          weight: 700
          style: italic
    - family: KaTeX_Math
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Math-Italic.ttf
          style: italic
        - asset: packages/ratex_flutter/fonts/KaTeX_Math-BoldItalic.ttf
          weight: 700
          style: italic
    - family: KaTeX_SansSerif
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_SansSerif-Regular.ttf
        - asset: packages/ratex_flutter/fonts/KaTeX_SansSerif-Bold.ttf
          weight: 700
        - asset: packages/ratex_flutter/fonts/KaTeX_SansSerif-Italic.ttf
          style: italic
    - family: KaTeX_Script
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Script-Regular.ttf
    - family: KaTeX_Typewriter
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Typewriter-Regular.ttf
    - family: KaTeX_Size1
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Size1-Regular.ttf
    - family: KaTeX_Size2
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Size2-Regular.ttf
    - family: KaTeX_Size3
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Size3-Regular.ttf
    - family: KaTeX_Size4
      fonts:
        - asset: packages/ratex_flutter/fonts/KaTeX_Size4-Regular.ttf
```

Without this step, `RaTeXPainter` silently falls back to the system font and formulas render incorrectly.

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

### Inline formula (mixed text + LaTeX)

Flutter's `RichText` + `WidgetSpan` is the recommended approach for mixing plain text with inline formulas. Use `PlaceholderAlignment.middle` for vertical centering:

```dart
import 'package:flutter/material.dart';
import 'package:ratex_flutter/ratex_flutter.dart';

/// Parses [text] with `$...$` inline math markers and returns a [RichText]
/// that intermixes plain [TextSpan]s with [WidgetSpan]s.
Widget buildInlineMath(String text, {double mathFontSize = 18, TextStyle? textStyle}) {
  final style = textStyle ??
      const TextStyle(fontSize: 16, height: 1.8, color: Colors.black87);

  final parts = text.split(r'$');
  final spans = <InlineSpan>[];

  for (int i = 0; i < parts.length; i++) {
    if (parts[i].isEmpty) continue;
    if (i.isEven) {
      // Plain text
      spans.add(TextSpan(text: parts[i], style: style));
    } else {
      // Inline math
      spans.add(WidgetSpan(
        alignment: PlaceholderAlignment.middle,
        baseline: TextBaseline.alphabetic,
        child: RaTeXWidget(
          latex: parts[i],
          fontSize: mathFontSize,
          onError: (e) => debugPrint('RaTeX inline error: $e'),
          loading: const SizedBox.shrink(),
        ),
      ));
    }
  }

  return RichText(text: TextSpan(children: spans));
}

// Usage:
buildInlineMath(
  r'质能等价关系 $E = mc^2$，其中光速 $c \approx 3\times10^8\ \text{m/s}$。',
)
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

   **CI**: Pushing a version tag (e.g. `v{VERSION}`) runs [release-flutter.yml](https://github.com/erweixin/RaTeX/blob/main/.github/workflows/release-flutter.yml): it builds Android and iOS native libs, injects them into this package, and runs `dart pub publish`. Ensure the tag matches the `version` in `pubspec.yaml`. Repository secret required: `PUB_DEV_TOKEN` (create at https://pub.dev/settings/tokens).
