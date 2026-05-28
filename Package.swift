// swift-tools-version: 5.9
//
// RaTeX — Native iOS LaTeX rendering via CoreGraphics + CoreText.
//
// Development (local):
//   1. Run `bash platforms/ios/build-ios.sh` to produce an iOS-only RaTeX.xcframework
//   2. Add this package locally in Xcode via File → Add Package Dependencies → Add Local…
//
// React Native macOS needs iOS + macOS slices:
//   `bash scripts/build-apple-xcframework.sh`
//
// Published releases use a remote binaryTarget (url + checksum).
// The CI workflow substitutes the path: target below before tagging a release.

import PackageDescription

let package = Package(
    name: "RaTeX",
    platforms: [.iOS(.v14)],
    products: [
        .library(name: "RaTeX", targets: ["RaTeX"]),
    ],
    targets: [
        // Pre-built XCFramework — iOS-only build entry:
        // `bash platforms/ios/build-ios.sh` (delegates to the unified Apple script).
        // In published releases this is replaced with a remote url + checksum target.
        .binaryTarget(
            name: "RaTeXFFI",
            url: "https://github.com/erweixin/RaTeX/releases/download/v0.1.10/RaTeX.xcframework.zip",
            checksum: "57ee4ca8973d566b1ad6c0e7fb277f63ca20aae16bc6b1a5d5a6da2bf2a31d26"
        ),

        // Swift wrapper: rendering, font loading, UIKit/SwiftUI views.
        .target(
            name: "RaTeX",
            dependencies: ["RaTeXFFI"],
            path: "platforms/ios/Sources/Ratex",
            resources: [
                // KaTeX 字体随包内置，ensureLoaded()/loadFromPackageBundle() 开箱即用
                .copy("Fonts"),
            ]
        ),
    ]
)
