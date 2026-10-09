// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "RingsealSDK",
    platforms: [
        .iOS(.v15)
    ],
    products: [
        .library(name: "RingsealSDK", targets: ["RingsealSDK"]),
    ],
    targets: [
        .target(
            name: "RingsealSDK",
            path: "Sources/RingsealSDK"
        ),
        .testTarget(
            name: "RingsealSDKTests",
            dependencies: ["RingsealSDK"],
            path: "Tests/RingsealSDKTests"
        ),
    ]
)
