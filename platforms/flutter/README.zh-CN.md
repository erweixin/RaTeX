# RaTeX — Flutter 集成说明

通过 Dart FFI 与 CustomPainter 在 Flutter 中原生渲染 LaTeX 数学公式。
无 WebView、无 JavaScript。

---

## 工作原理

```
LaTeX 字符串
    ↓ RaTeXFfi.parseAndLayout()   [Dart FFI → libratex_ffi]
JSON DisplayList
    ↓ DisplayList.fromJson()       [Dart JSON 解码]
DisplayList
    ↓ RaTeXPainter.paint()         [flutter/canvas]
CustomPaint Widget
```

---

## 开箱即用

1. **添加依赖** — 在 `pubspec.yaml` 中：`ratex_flutter: ^0.1.0`，然后执行 `flutter pub get`。无需自行编译原生库，发布包内已含 Android `.so` 与 iOS XCFramework。
2. **注册字体** — Flutter 不会自动为宿主应用注册插件字体。请将 [KaTeX 字体声明](#字体配置) 复制到你的 `pubspec.yaml`（见下方安装说明）。
3. **使用** — 直接使用 `RaTeXWidget`：
   ```dart
   RaTeXWidget(
     latex: r'\frac{-b \pm \sqrt{b^2-4ac}}{2a}',
     fontSize: 28,
     onError: (e) => debugPrint('RaTeX: $e'),
   )
   ```

---

## 安装

### 从 pub.dev（推荐）

在 `pubspec.yaml` 中添加：

```yaml
dependencies:
  ratex_flutter: ^0.1.0
```

然后执行 `flutter pub get`。无需本地构建 — 已发布包内含预编译的 Android `.so` 与 iOS `RaTeX.xcframework`。

#### 字体配置

Flutter 要求宿主应用显式声明来自插件包的字体（[Flutter 文档](https://docs.flutter.dev/cookbook/design/package-fonts#from-a-package)）。请将以下内容添加到 `pubspec.yaml` 的 `flutter:` 节：

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

若跳过此步骤，`RaTeXPainter` 会静默回退到系统字体，公式渲染将出现异常。

### 从本地路径（开发）

若从 RaTeX 仓库使用该包：

```yaml
dependencies:
  ratex_flutter:
    path: /path/to/RaTeX/platforms/flutter
```

需先构建原生库：

| 平台 | 命令 |
|------|------|
| iOS | `bash platforms/ios/build-ios.sh`（生成 `RaTeX.xcframework`） |
| Android | `bash platforms/android/build-android.sh`（生成 `.so` 文件） |

**从源码构建的环境要求**：Flutter 3.10+、Dart 3.0+、Rust 1.75+。

---

## 使用

### Widget（推荐）

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

### 底层 CustomPainter

```dart
import 'package:ratex_flutter/ratex_flutter.dart';

final dl      = RaTeXEngine.instance.parseAndLayout(r'\sum_{n=1}^\infty \frac{1}{n^2}');
final painter = RaTeXPainter(displayList: dl, fontSize: 24);

// 在 CustomPaint 中：
CustomPaint(painter: painter, size: Size(painter.widthPx, painter.totalHeightPx))
```

### 行内公式（文字 + LaTeX 混排）

Flutter 推荐使用 `RichText` + `WidgetSpan` 实现行内公式混排。使用 `PlaceholderAlignment.middle` 进行垂直居中对齐：

```dart
import 'package:flutter/material.dart';
import 'package:ratex_flutter/ratex_flutter.dart';

/// 解析含 `$...$` 行内数学标记的 [text]，返回交织了普通 [TextSpan] 与 [WidgetSpan] 的 [RichText]。
Widget buildInlineMath(String text, {double mathFontSize = 18, TextStyle? textStyle}) {
  final style = textStyle ??
      const TextStyle(fontSize: 16, height: 1.8, color: Colors.black87);

  final parts = text.split(r'$');
  final spans = <InlineSpan>[];

  for (int i = 0; i < parts.length; i++) {
    if (parts[i].isEmpty) continue;
    if (i.isEven) {
      // 普通文本
      spans.add(TextSpan(text: parts[i], style: style));
    } else {
      // 行内公式
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

// 用法：
buildInlineMath(
  r'质能等价关系 $E = mc^2$，其中光速 $c \approx 3\times10^8\ \text{m/s}$。',
)
```

### 异步（大公式）

```dart
import 'package:flutter/foundation.dart';

final dl = await compute(
  (latex) => RaTeXEngine.instance.parseAndLayout(latex),
  r'\prod_{n=1}^\infty \left(1 - \frac{1}{n^2}\right)',
);
```

---

## 坐标系

与 iOS/Android 一致：所有坐标为 **em 单位**，乘以 `fontSize`（逻辑像素）得到屏幕坐标。Y 自边界框顶部向下递增。基线位于 Y = `height × fontSize`。

---

## 文件说明

| 文件 | 说明 |
|------|------|
| `pubspec.yaml` | Flutter 插件清单 |
| `ios/` | iOS 插件（podspec + RaTeXPlugin.swift）；链接 RaTeX.xcframework |
| `android/` | Android 插件（RaTeXPlugin.kt）；使用包内 `jniLibs` 中的 `libratex_ffi.so` |
| `lib/ratex_flutter.dart` | 对外 API：`RaTeXEngine`、`RaTeXWidget` |
| `lib/src/display_list.dart` | Dart JSON 类型（DisplayList、DisplayItem 等） |
| `lib/src/ratex_ffi.dart` | 对 `libratex_ffi` 的 Dart FFI 绑定 |
| `lib/src/ratex_painter.dart` | `CustomPainter` 绘制循环 |

---

## 发布到 pub.dev（维护者）

要发布**开箱即用**、无需用户构建原生代码的包：

1. **Android** — 构建并将 JNI 库复制到包内：
   ```bash
   # 在仓库根目录
   ./platforms/android/build-android.sh
   cp -R platforms/android/src/main/jniLibs/* platforms/flutter/android/src/main/jniLibs/
   ```

2. **iOS** — 确保 `RaTeX.xcframework` 在包内为实体目录（非符号链接）：
   ```bash
   # 在仓库根目录
   ./platforms/ios/build-ios.sh
   # 若 platforms/flutter/ios/RaTeX.xcframework 为符号链接，替换为实体拷贝：
   rm -rf platforms/flutter/ios/RaTeX.xcframework
   cp -R platforms/ios/RaTeX.xcframework platforms/flutter/ios/
   ```

3. **校验并发布**：
   ```bash
   cd platforms/flutter
   dart pub publish --dry-run
   dart pub publish
   ```

   **CI**：推送版本 tag（如 `v{VERSION}`）会触发 [release-flutter.yml](https://github.com/erweixin/RaTeX/blob/main/.github/workflows/release-flutter.yml)：构建 Android 与 iOS 原生库、注入本包并执行 `dart pub publish`。请确保 tag 与 `pubspec.yaml` 中的 `version` 一致。仓库需配置 Secret：`PUB_DEV_TOKEN`（在 https://pub.dev/settings/tokens 创建）。
