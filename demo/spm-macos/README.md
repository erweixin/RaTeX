# RaTeX SPM macOS Demo

This demo verifies RaTeX as a macOS Swift Package Manager dependency. It uses a local path dependency on the repository root, so it exercises the same `Package.swift` and `RaTeX.xcframework` binary target that a macOS SPM consumer uses.

## Run

From the repository root:

```bash
bash scripts/build-apple-xcframework.sh
swift run --package-path demo/spm-macos
```

The first command is required for local development because `Package.swift` points to `platforms/ios/RaTeX.xcframework`, and that local XCFramework must contain a macOS slice.

## What It Checks

- `Package.swift` exposes RaTeX on macOS.
- `RaTeXFFI` resolves from the local XCFramework through SPM.
- `RaTeXEngine.parse` works from a macOS app.
- `RaTeXFormula` renders through SwiftUI using `NSViewRepresentable`.
