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

### Option A — Swift Package (recommended)

In Xcode: **File → Add Package Dependencies** → point to the local path
`platforms/ios/` (or your fork URL). Select the `RaTeX` product.

### Option B — Manual

1. Drag `RaTeX.xcframework` into your Xcode project.
2. In **Build Phases → Link Binary With Libraries**, ensure it is listed.
3. Copy the `Sources/RaTeX/*.swift` files into your project.

---

## Usage

### UIKit

```swift
import RaTeX

let mathView = RaTeXView()
mathView.latex    = #"\frac{-b \pm \sqrt{b^2-4ac}}{2a}"#
mathView.fontSize = 28
mathView.onError  = { print("RaTeX error:", $0) }

// Auto-sizing
mathView.translatesAutoresizingMaskIntoConstraints = false
view.addSubview(mathView)
NSLayoutConstraint.activate([
    mathView.centerXAnchor.constraint(equalTo: view.centerXAnchor),
    mathView.centerYAnchor.constraint(equalTo: view.centerYAnchor),
])
```

### SwiftUI

```swift
import RaTeX

struct ContentView: View {
    var body: some View {
        RaTeXFormula(
            latex: #"\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}"#,
            fontSize: 24
        )
        .padding()
    }
}
```

### Low-level (custom drawing)

```swift
import RaTeX

let displayList = try RaTeXEngine.shared.parse(#"\sum_{n=1}^\infty \frac{1}{n^2}"#)
let renderer    = RaTeXRenderer(displayList: displayList, fontSize: 20)

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
