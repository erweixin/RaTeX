# RaTeX — iOS Integration Guide

Native iOS rendering of LaTeX math formulas via Swift and CoreGraphics.
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
UIView / SwiftUI View
```

---

## Out of the box

1. **Add dependency** — via Swift Package Manager (see [Installation](#add-to-your-xcode-project) below).
2. **Use** — Use `RaTeXView` or `RaTeXFormula`; fonts load automatically on first render.
   ```swift
   // SwiftUI
   RaTeXFormula(latex: #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#, fontSize: 24)
   ```
   **Optional:** To preload fonts at startup, call `RaTeXFontLoader.loadFromPackageBundle()` when the app launches.

---

## Prerequisites

| Tool | Version |
|------|---------|
| Xcode | 15+ |
| Rust | 1.75+ (`rustup`) |
| iOS target | 14+ |

Install Rust iOS targets once:

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

---

## Build the XCFramework

From the repo root:

```bash
bash platforms/ios/build-ios.sh
```

This produces `platforms/ios/RaTeX.xcframework`.

---

## Add to your Xcode project

### Option A — Swift Package Manager (recommended)

In Xcode: **File → Add Package Dependencies**, enter `https://github.com/erweixin/RaTeX` and select the `RaTeX` product.

**Local development** — Run `bash platforms/ios/build-ios.sh` first, then point Xcode to the repo root via **File → Add Package Dependencies → Add Local…**.

### Option B — Manual

1. Drag `platforms/ios/RaTeX.xcframework` into your Xcode project.
2. In **Build Phases → Link Binary With Libraries**, ensure it is listed.
3. Copy the `platforms/ios/Sources/RaTeX/*.swift` files into your project.
4. Add the `Fonts` folder from `platforms/ios/Sources/Ratex/Fonts/` to your target’s **Copy Bundle Resources**; fonts load automatically on first render, or call `RaTeXFontLoader.loadFromBundle()` at startup.

---

## Usage

### UIKit

```swift
import RaTeX

let mathView = RaTeXView()
mathView.latex       = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
mathView.fontSize    = 28
mathView.displayMode = true   // true = display/block (default); false = inline/text
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
            fontSize: 24
            // displayMode: true  ← default; pass false for inline/text style
        )
        .padding()
    }
}
```

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

### Low-level (custom drawing)

```swift
import RaTeX

// display mode (default)
let displayList = try RaTeXEngine.shared.parse(#"\sum_{n=1}^\infty \frac{1}{n^2}"#)
// inline mode
let displayList = try RaTeXEngine.shared.parse(#"\frac{1}{2}"#, displayMode: false)

let renderer = RaTeXRenderer(displayList: displayList, fontSize: 20)

// In your UIView.draw(_:) or CGContext block:
renderer.draw(in: UIGraphicsGetCurrentContext()!)
```

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
| `build-ios.sh` | Build script → `RaTeX.xcframework` |
| `Package.swift` | Swift Package manifest |
| `Sources/RaTeX/DisplayList.swift` | Codable Swift mirror of Rust types |
| `Sources/RaTeX/RaTeXEngine.swift` | Calls C ABI, decodes JSON |
| `Sources/RaTeX/RaTeXRenderer.swift` | CoreGraphics drawing loop |
| `Sources/RaTeX/RaTeXView.swift` | UIKit `UIView` + SwiftUI `View` |
