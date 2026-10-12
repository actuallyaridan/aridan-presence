// swift-tools-version: 6.2

// The macOS app, in SwiftUI. build.sh builds the Rust side first (src/,
// into ../target/release), then this, then puts the two together into
// aridan-presence.app.

import Foundation
import PackageDescription

// Where cargo leaves libaridan_presence_macos.a.
let rustLibrary = URL(fileURLWithPath: #filePath)
    .deletingLastPathComponent()
    .appendingPathComponent("../target/release")
    .standardizedFileURL
    .path

let package = Package(
    name: "aridan-presence",
    platforms: [.macOS(.v26)],
    targets: [
        // The Rust side's functions, from Sources/CPresence/presence.h.
        .systemLibrary(name: "CPresence"),

        .executableTarget(
            name: "Presence",
            dependencies: ["CPresence"],
            linkerSettings: [
                .unsafeFlags(["-L", rustLibrary]),

                // What the Rust side uses from macOS, as
                // `cargo rustc -- --print native-static-libs` lists it:
                // HTTPS through the Security framework, and the computer's
                // name and processes through IOKit and OpenDirectory.
                .linkedFramework("CoreFoundation"),
                .linkedFramework("Foundation"),
                .linkedFramework("IOKit"),
                .linkedFramework("OpenDirectory"),
                .linkedFramework("Security"),
                .linkedLibrary("iconv"),
                .linkedLibrary("objc"),
            ]
        ),
    ]
)
