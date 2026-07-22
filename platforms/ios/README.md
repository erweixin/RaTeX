# RaTeX — Apple Integration Guide

Native iOS and macOS rendering of LaTeX math formulas via Swift and CoreGraphics.
No WebView, no JavaScript, no DOM.

---

## How it works

```
LaTeX string
    ↓ ratex_parse_and_layout() [C ABI, static lib]
JSON DisplayList
    ↓ RaTeXEngine.parse()       [Swift JSON decode]
DisplayList
    ↓ RaTeXRenderer.draw()      [CoreGraphics]
UIView / NSView / SwiftUI View
```

---

## Out of the box

1. **Add the dependency** — use Swift Package Manager for the complete native UI
   package, or CocoaPods for the core-only renderer (see
   [Installation](#add-to-your-xcode-project)).
2. **Choose the API that matches the dependency:**
   - **Swift Package Manager:** use `RaTeXView` or `RaTeXFormula`.

     ```swift
     // SwiftUI
     RaTeXFormula(latex: #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#, fontSize: 24)
     ```

   - **RaTeXCore CocoaPod:** use `RaTeXEngine` and `RaTeXRenderer` from your
     custom `UIView`, `NSView`, or `CGContext` drawing code.

     ```swift
     let displayList = try RaTeXEngine.shared.parse(#"\frac{1}{2}"#)
     let renderer = RaTeXRenderer(displayList: displayList, fontSize: 20)
     renderer.draw(in: context)
     ```

Fonts load automatically on the first `RaTeXRenderer.draw(in:)` call. To preload
them at startup, use `RaTeXFontLoader.loadFromPackageBundle()` with Swift Package
Manager or `RaTeXFontLoader.loadFromCocoaPodsBundle()` with `RaTeXCore`.

---

## Prerequisites

| Tool | Version |
|------|---------|
| Xcode | 15+ |
| Rust | 1.75+ (`rustup`) |
| iOS target | 14+ |
| macOS target | 14+ |

Install Rust Apple targets once:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

---

## Build the XCFramework

From the repo root:

```bash
bash scripts/build-apple-xcframework.sh
```

This produces `platforms/ios/RaTeX.xcframework` with **iOS** and **macOS** slices.

For an iOS-only local artifact, use the compatibility wrapper:

```bash
bash platforms/ios/build-ios.sh
```

---

## Add to your Xcode project

### Option A — Swift Package Manager (recommended)

In Xcode: **File → Add Package Dependencies**, enter `https://github.com/erweixin/RaTeX` and select the `RaTeX` product.

**Local development** — Run `bash platforms/ios/build-ios.sh` first for iOS-only work, or `bash scripts/build-apple-xcframework.sh` for macOS SPM consumers. Then point Xcode to the repo root via **File → Add Package Dependencies → Add Local…**.

For a runnable macOS SPM integration check, see `demo/spm-macos`.

### Option B — Manual

1. Drag `platforms/ios/RaTeX.xcframework` into your Xcode project.
2. In **Build Phases → Link Binary With Libraries**, ensure it is listed.
3. Copy the `platforms/ios/Sources/Ratex/*.swift` files into your project, including `PlatformCompat.swift`.
4. Add the `Fonts` folder from `platforms/ios/Sources/Ratex/Fonts/` to your target’s **Copy Bundle Resources**; fonts load automatically on first render, or call `RaTeXFontLoader.loadFromBundle()` at startup.

### Option C — CocoaPods (core only)

Add the repository pod to your `Podfile`:

```ruby
pod "RaTeXCore", :git => "https://github.com/erweixin/RaTeX.git", :branch => "main"
```

For reproducible builds, replace `:branch => "main"` with a released RaTeX
tag after the first release containing `RaTeXCore.podspec`.

```swift
import RaTeXCore

let displayList = try RaTeXEngine.shared.parse(#"\frac{1}{2}"#)
let renderer = RaTeXRenderer(displayList: displayList, fontSize: 20)

// In your UIView.draw(_:) or CGContext block:
renderer.draw(in: context)
```

`RaTeXCore` is a static, UI-free target. It contains the FFI XCFramework,
display-list types, engine, renderer, font loader, and KaTeX font bundle, but
does not depend on React, React Native, Fabric, Codegen, or JavaScript.

---

## Usage

### UIKit

```swift
import RaTeX

let mathView = RaTeXView()
mathView.latex       = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
mathView.fontSize    = 28
mathView.displayMode = true   // true = display/block (default); false = inline/text
mathView.color       = .systemBlue
mathView.onError     = { print("RaTeX error:", $0) }

// Auto-sizing
mathView.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(mathView)
NSLayoutConstraint.activate([
    mathView.centerXAnchor.constraint(equalTo: view.centerXAnchor),
    mathView.centerYAnchor.constraint(equalTo: view.centerYAnchor),
])
```

### SwiftUI — block formula

```swift
import RaTeX

struct ContentView: View {
    var body: some View {
        RaTeXFormula(
            latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#,
            fontSize: 24,
            color: .blue
            // displayMode: true  ← default; pass false for inline/text style
        )
        .ratexColor(.primary)
        .padding()
    }
}
```

Use `.ratexColor(...)` to set a default color for descendant formulas, and pass `color:` to override an individual `RaTeXFormula`.

### SwiftUI — inline formula (mixed text + LaTeX)

For inline rendering, use a custom `FlowLayout` (a SwiftUI `Layout`) that places `Text` and `RaTeXFormula` children side-by-side with automatic line wrapping. Baseline alignment uses the library-provided `RaTeXFormulaAscentKey` layout value, which `RaTeXFormula` exposes from the first frame — no two-pass measurement needed.

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

// FlowLayout: wrap children horizontally, align baselines.
// Reads RaTeXFormulaAscentKey for formula baseline; falls back to
// firstTextBaseline for Text views.
struct FlowLayout: Layout {
    var horizontalSpacing: CGFloat = 4
    var lineSpacing: CGFloat = 6

    // ... see demo/ios for the full implementation
}
```

`RaTeXFormulaAscentKey` is a `LayoutValueKey<CGFloat>` built into the library. It carries the formula's ascent (distance from baseline to top) so that `FlowLayout` can align mixed children without manual offset calculation.

> **Baseline alignment is cross-platform.** `RaTeXFormula` reports its math baseline through `.alignmentGuide(.firstTextBaseline)`, so `HStack(alignment: .firstTextBaseline)` and `Text` baseline alignment line up correctly on **both iOS and macOS**, on every supported OS version — no custom `Layout` required for the simple case:
>
> ```swift
> HStack(alignment: .firstTextBaseline) {
>     Text("Euler's identity:")
>     RaTeXFormula(latex: #"e^{i\pi}+1=0"#, fontSize: 17, displayMode: false)
> }
> ```
>
> The `RaTeXFormulaAscentKey` + custom `Layout` approach above (iOS 16+ / macOS 13+) is only needed when you also want automatic line wrapping; see `demo/spm-macos` for a runnable macOS example. When embedding `RaTeXView` directly in raw UIKit/AppKit Auto Layout, its `firstBaselineAnchor` / `lastBaselineAnchor` already resolve to the formula's math baseline on **both platforms** — UIKit via `forFirstBaselineLayout`, AppKit via `firstBaselineOffsetFromTop` / `lastBaselineOffsetFromBottom` — so a baseline constraint against an `NSTextField`/`UILabel` lines up without any extra work.

### Low-level (custom drawing)

```swift
import RaTeX

// display mode (default)
let displayList = try RaTeXEngine.shared.parse(#"\sum_{n=1}^\infty \frac{1}{n^2}"#)
// inline mode
let displayList = try RaTeXEngine.shared.parse(#"\frac{1}{2}"#, displayMode: false)
// custom default color
let blueDisplayList = try RaTeXEngine.shared.parse(#"x + y"#, color: .systemBlue)

let renderer = RaTeXRenderer(displayList: displayList, fontSize: 20)

// In your UIView.draw(_:) or CGContext block:
renderer.draw(in: UIGraphicsGetCurrentContext()!)
```

Explicit LaTeX colors such as `\color{...}` still override the default color for their subtree.

---

## Coordinate system

All `DisplayList` coordinates are in **em units**. `RaTeXRenderer` multiplies them
by `fontSize` (pt) to produce screen coordinates.

- X increases rightward from the left edge.
- Y increases downward from the top edge.
- Baseline is at Y = `height × fontSize`.

---

## File map

| File | Purpose |
|------|---------|
| `build-ios.sh` | iOS-only build entry (delegates to unified Apple build script) |
| `Package.swift` | Swift Package manifest |
| `Sources/Ratex/DisplayList.swift` | Codable Swift mirror of Rust types |
| `Sources/Ratex/PlatformCompat.swift` | UIKit/AppKit compatibility aliases |
| `Sources/Ratex/RaTeXEngine.swift` | Calls C ABI, decodes JSON |
| `Sources/Ratex/RaTeXRenderer.swift` | CoreGraphics drawing loop |
| `Sources/Ratex/RaTeXView.swift` | UIKit/AppKit view + SwiftUI `View` |
