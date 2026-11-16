// swift-tools-version: 5.9
//
// RaTeX — Native Apple LaTeX rendering via CoreGraphics + CoreText.
//
// Development (local):
//   1. Run `bash scripts/build-apple-xcframework.sh` to produce an iOS + macOS + Mac Catalyst RaTeX.xcframework
//   2. Add this package locally in Xcode via File → Add Package Dependencies → Add Local…
//
// Published releases use a remote binaryTarget (url + checksum).
// The CI workflow substitutes the path: target below before tagging a release.

import PackageDescription

let package = Package(
    name: "RaTeX",
    platforms: [.iOS(.v14), .macOS(.v14), .macCatalyst(.v14)],
    products: [
        .library(name: "RaTeX", targets: ["RaTeX"]),
    ],
    targets: [
        // Pre-built XCFramework - iOS + macOS + Mac Catalyst build entry:
        // `bash scripts/build-apple-xcframework.sh`.
        // In published releases this is replaced with a remote url + checksum target.
        .binaryTarget(
            name: "RaTeXFFI",
            url: "https://github.com/erweixin/RaTeX/releases/download/v0.1.16/RaTeX.xcframework.zip",
            checksum: "c16286331d20c7daf7907878b17c81562c480d6cdb1ab2a6d7d0788d0d4ba18e"
        ),

        // Swift wrapper: rendering, font loading, UIKit/AppKit/SwiftUI views.
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
