// swift-tools-version: 5.9
// platforms/ios/tests — macOS test package for the RaTeX Swift binding layer.
//
// Run tests (from repo root):
//   LIBRARY_PATH=$(pwd)/target/release swift test --package-path platforms/ios/tests
//
// (The LIBRARY_PATH tells the linker where to find libratex_ffi.a/dylib)

import PackageDescription

let package = Package(
    name: "RaTeXSwiftPackage",
    platforms: [.macOS(.v13)],
    products: [
        .library(name: "RaTeXCore", targets: ["RaTeXCore"]),
        .executable(name: "ratex-smoke", targets: ["RaTeXSmoke"]),
    ],
    targets: [
        // C system library — just exposes the header; links libratex_ffi via module.modulemap
        .systemLibrary(
            name: "CRaTeX",
            path: "Sources/CRaTeX"
        ),

        // Swift wrapper (no UIKit, macOS-compatible)
        .target(
            name: "RaTeXCore",
            dependencies: ["CRaTeX"],
            path: "Sources/RaTeXCore"
        ),

        // Smoke test executable (runs on macOS; links the same C ABI).
        .executableTarget(
            name: "RaTeXSmoke",
            dependencies: ["RaTeXCore"],
            path: "Sources/RaTeXSmoke"
        ),

        // Tests
        .testTarget(
            name: "RaTeXCoreTests",
            dependencies: ["RaTeXCore"],
            path: "Tests/RaTeXCoreTests"
        ),
    ]
)
