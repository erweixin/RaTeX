// swift-tools-version: 5.9
//
// RaTeX — Native iOS LaTeX rendering via CoreGraphics + CoreText.
//
// Development (local):
//   1. Run `bash platforms/ios/build-ios.sh` to produce RaTeX.xcframework
//   2. Add this package locally in Xcode via File → Add Package Dependencies → Add Local…
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
        // Pre-built XCFramework — generate with `bash platforms/ios/build-ios.sh`.
        // In published releases this is replaced with a remote url + checksum target.
        .binaryTarget(
            name: "RaTeXFFI",
            url: "https://github.com/erweixin/RaTeX/releases/download/v0.0.4/RaTeX.xcframework.zip",
            checksum: "3273ab29d287763451b1512a045a2db3096a7647801f39436a5a0f90d70c62c1"
        ),

        // Swift wrapper: rendering, font loading, UIKit/SwiftUI views.
        .target(
            name: "RaTeX",
            dependencies: ["RaTeXFFI"],
            path: "platforms/ios/Sources/RaTeX",
            resources: [
                // KaTeX fonts — loaded automatically via RaTeXFontLoader.loadFromPackageBundle()
                .copy("Fonts"),
            ]
        ),
    ]
)
