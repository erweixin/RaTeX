// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "RaTeXSPMMacOSDemo",
    platforms: [.macOS(.v14)],
    dependencies: [
        .package(path: "../.."),
    ],
    targets: [
        .executableTarget(
            name: "RaTeXSPMMacOSDemo",
            dependencies: [
                .product(name: "RaTeX", package: "RaTeX"),
            ]
        ),
    ]
)
