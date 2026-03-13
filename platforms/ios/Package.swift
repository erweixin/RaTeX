// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RaTeX",
    platforms: [.iOS(.v14)],
    products: [
        .library(name: "RaTeX", targets: ["RaTeX"]),
    ],
    targets: [
        // Pre-built XCFramework (run build-ios.sh to generate)
        .binaryTarget(
            name: "RaTeXFFI",
            path: "RaTeX.xcframework"
        ),
        // Swift wrapper
        .target(
            name: "RaTeX",
            dependencies: ["RaTeXFFI"],
            path: "Sources/RaTeX"
        ),
    ]
)
