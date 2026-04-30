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
            url: "https://github.com/erweixin/RaTeX/releases/download/v0.1.4/RaTeX.xcframework.zip",
            checksum: "b1ea8dd07313e445697852b0d3ce221d9956b819598252353179c5dd412bedb8"
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
