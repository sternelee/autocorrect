// swift-tools-version: 5.9

import PackageDescription

let package = Package(
    name: "AppleTranslation",
    platforms: [.macOS(.v14)],
    products: [
        .library(name: "AppleTranslation", type: .static, targets: ["AppleTranslation"]),
    ],
    targets: [
        .target(name: "AppleTranslation"),
    ]
)
