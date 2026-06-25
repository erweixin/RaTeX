# RaTeX — Apple 集成说明

通过 Swift 与 CoreGraphics 在 iOS 和 macOS 上原生渲染 LaTeX 数学公式。
无 WebView、无 JavaScript、无 DOM。

---

## 工作原理

```
LaTeX 字符串
    ↓ ratex_parse_and_layout() [C ABI，静态库]
JSON DisplayList
    ↓ RaTeXEngine.parse()       [Swift JSON 解码]
DisplayList
    ↓ RaTeXRenderer.draw()      [CoreGraphics]
UIView / NSView / SwiftUI View
```

---

## 开箱即用

1. **添加依赖** — Xcode：**File → Add Package Dependencies**，输入仓库 URL，选择 `RaTeX`。
2. **使用** — 直接使用 `RaTeXView` 或 `RaTeXFormula`，字体会在首次渲染时自动加载，无需手动调用。
   ```swift
   // SwiftUI
   RaTeXFormula(latex: #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#, fontSize: 24)
   ```
   **可选**：若希望在启动时提前加载字体（例如避免首屏公式略有延迟），可在 App 启动时调用 `RaTeXFontLoader.loadFromPackageBundle()`。

**本地开发**（修改 RaTeX 源码时）：iOS-only 可先执行 `bash platforms/ios/build-ios.sh`；如果要验证 macOS SPM 集成，请执行 `bash scripts/build-apple-xcframework.sh`，再在 Xcode 里 **File → Add Package Dependencies → Add Local…** 选择 RaTeX 仓库根目录。

---

## 环境要求

| 工具 | 版本 |
|------|------|
| Xcode | 15+ |
| Rust | 1.75+（rustup） |
| iOS 目标 | 14+ |
| macOS 目标 | 14+ |

安装 Rust Apple 目标（一次性）：

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

---

## 构建 XCFramework

在仓库根目录执行：

```bash
bash scripts/build-apple-xcframework.sh
```

将生成同时包含 **iOS** 和 **macOS** 切片的 `platforms/ios/RaTeX.xcframework`。

如果只需要本地 iOS 产物，可使用兼容入口：

```bash
bash platforms/ios/build-ios.sh
```

---

## 接入 Xcode 项目

### 方式 A — Swift Package Manager（推荐）

在 Xcode 中：**File → Add Package Dependencies**，输入 `https://github.com/erweixin/RaTeX`，选择 `RaTeX` 产品。

**本地开发** — iOS-only 可先执行 `bash platforms/ios/build-ios.sh`；macOS SPM 消费方请执行 `bash scripts/build-apple-xcframework.sh`，确保本地 binary target 包含 macOS 切片。然后在 Xcode 中通过 **File → Add Package Dependencies → Add Local…** 指向仓库根目录。

可运行的 macOS SPM 集成验证示例见 `demo/spm-macos`。

### 方式 B — 手动集成

1. 将 `platforms/ios/RaTeX.xcframework` 拖入 Xcode 项目。
2. 在 **Build Phases → Link Binary With Libraries** 中确保已链接。
3. 将 `platforms/ios/Sources/Ratex/*.swift` 复制到你的项目中，包括 `PlatformCompat.swift`。
4. 将 `platforms/ios/Sources/Ratex/Fonts/` 下的 `Fonts` 文件夹加入 target 的 **Copy Bundle Resources**；字体会在首次渲染时自动加载，或在启动时调用 `RaTeXFontLoader.loadFromBundle()`。

---

## 使用

### UIKit

```swift
import RaTeX

let mathView = RaTeXView()
mathView.latex       = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
mathView.fontSize    = 28
mathView.displayMode = true   // true = 独立块（默认）；false = 行内
mathView.color       = .systemBlue
mathView.onError     = { print("RaTeX error:", $0) }

// 自动布局
mathView.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(mathView)
NSLayoutConstraint.activate([
    mathView.centerXAnchor.constraint(equalTo: view.centerXAnchor),
    mathView.centerYAnchor.constraint(equalTo: view.centerYAnchor),
])
```

### SwiftUI — 块级公式

```swift
import RaTeX

struct ContentView: View {
    var body: some View {
        RaTeXFormula(
            latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#,
            fontSize: 24,
            color: .blue
            // displayMode: true  ← 默认独立块；传 false 切换为行内样式
        )
        .ratexColor(.primary)
        .padding()
    }
}
```

可使用 `.ratexColor(...)` 为一组公式设置默认颜色，也可以通过 `color:` 单独覆盖某个 `RaTeXFormula` 的颜色。

### SwiftUI — 行内公式（文字 + LaTeX 混排）

使用自定义 `FlowLayout`（SwiftUI `Layout`）将 `Text` 与 `RaTeXFormula` 并排排列，自动换行。基线对齐通过库内置的 `RaTeXFormulaAscentKey` 布局值实现，无需两次测量。

```swift
import RaTeX

struct InlineExample: View {
    private let fs: CGFloat = 17

    var body: some View {
        FlowLayout(horizontalSpacing: 3, lineSpacing: 6) {
            Text("由勾股定理")
            RaTeXFormula(latex: #"a^2 + b^2 = c^2"#, fontSize: fs, onError: { _ in })
            Text("可直接求得斜边长度。")
        }
    }
}

// FlowLayout：水平排列子视图，自动换行，基线对齐。
// 读取 RaTeXFormulaAscentKey 获取公式基线；Text 视图回退到 firstTextBaseline。
struct FlowLayout: Layout {
    var horizontalSpacing: CGFloat = 4
    var lineSpacing: CGFloat = 6

    // ... 完整实现见 demo/ios
}
```

`RaTeXFormulaAscentKey` 是库内置的 `LayoutValueKey<CGFloat>`，携带公式的 ascent（基线到顶部的距离），供 `FlowLayout` 在混排时对齐，无需手动计算偏移量。

> **基线对齐跨平台可用。** `RaTeXFormula` 通过 `.alignmentGuide(.firstTextBaseline)` 上报数学基线，因此 `HStack(alignment: .firstTextBaseline)` 与 `Text` 基线对齐在 **iOS 与 macOS 上、所有受支持的系统版本**都能正确对齐——简单场景无需自定义 `Layout`：
>
> ```swift
> HStack(alignment: .firstTextBaseline) {
>     Text("欧拉恒等式：")
>     RaTeXFormula(latex: #"e^{i\pi}+1=0"#, fontSize: 17, displayMode: false)
> }
> ```
>
> 上面基于 `RaTeXFormulaAscentKey` + 自定义 `Layout` 的方案（iOS 16+ / macOS 13+）仅在需要自动换行时才需要，可运行示例见 `demo/spm-macos`。若将 `RaTeXView` 直接嵌入原生 UIKit/AppKit Auto Layout，请使用 `firstBaselineAnchor`（iOS）或重写 `firstBaselineOffsetFromTop`（macOS）。

### 底层自定义绘制

```swift
import RaTeX

// 独立块（默认）
let displayList = try RaTeXEngine.shared.parse(#"\sum_{n=1}^\infty \frac{1}{n^2}"#)
// 行内
let displayList = try RaTeXEngine.shared.parse(#"\frac{1}{2}"#, displayMode: false)
// 自定义默认颜色
let blueDisplayList = try RaTeXEngine.shared.parse(#"x + y"#, color: .systemBlue)

let renderer = RaTeXRenderer(displayList: displayList, fontSize: 20)

// 在你的 UIView.draw(_:) 或 CGContext 块中：
renderer.draw(in: UIGraphicsGetCurrentContext()!)
```

显式 LaTeX 颜色（如 `\color{...}`）仍会覆盖外部传入的默认颜色。

---

## 坐标系

所有 `DisplayList` 坐标均为 **em 单位**。`RaTeXRenderer` 乘以 `fontSize`（pt）得到屏幕坐标。

- X 自左边缘向右递增。
- Y 自顶部向下递增。
- 基线位于 Y = `height × fontSize`。

---

## 文件说明

| 文件 | 说明 |
|------|------|
| `build-ios.sh` | iOS-only 构建入口（委托统一 Apple 构建脚本） |
| `Package.swift` | Swift Package 清单 |
| `Sources/Ratex/DisplayList.swift` | Rust 类型的 Codable Swift 镜像 |
| `Sources/Ratex/PlatformCompat.swift` | UIKit/AppKit 兼容别名 |
| `Sources/Ratex/RaTeXEngine.swift` | 调用 C ABI、解码 JSON |
| `Sources/Ratex/RaTeXRenderer.swift` | CoreGraphics 绘制循环 |
| `Sources/Ratex/RaTeXView.swift` | UIKit/AppKit view 与 SwiftUI `View` |
