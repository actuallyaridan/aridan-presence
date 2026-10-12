// Pictures for the window: an app's own icon, found by the name its
// activity is shared under, and covers, downloaded once and kept.

import AppKit
import Observation

@MainActor
enum AppIcons {
    // The names activities are shared under, and the apps they come from.
    // Anything else is looked for as "<name>.app".
    private static let bundles: [(name: String, id: String)] = [
        ("apple music", "com.apple.Music"),
        ("cider", "sh.cider.genten.mac"),
        ("spotify", "com.spotify.client"),
        ("firefox", "org.mozilla.firefox"),
        ("chrome", "com.google.Chrome"),
        ("edge", "com.microsoft.edgemac"),
        ("safari", "com.apple.Safari"),
        ("discord", "com.hnc.Discord"),
        ("steam", "com.valvesoftware.steam"),
        ("visual studio code", "com.microsoft.VSCode"),
        ("xcode", "com.apple.dt.Xcode"),
    ]

    private static var cache: [String: NSImage?] = [:]

    // The app's icon, or nil if it is not on this Mac.
    static func icon(for name: String) -> NSImage? {
        let lower = name.lowercased()
        if let cached = cache[lower] { return cached }

        let icon = find(lower, name)
        cache[lower] = .some(icon)
        return icon
    }

    private static func find(_ lower: String, _ name: String) -> NSImage? {
        let workspace = NSWorkspace.shared

        for bundle in bundles where lower.contains(bundle.name) {
            if let url = workspace.urlForApplication(withBundleIdentifier: bundle.id) {
                return workspace.icon(forFile: url.path)
            }
        }

        // Cider, without knowing its bundle ID for sure, and most other apps:
        // "Steam" is Steam.app.
        let folders = [
            "/Applications",
            "/System/Applications",
            NSHomeDirectory() + "/Applications",
        ]
        for folder in folders {
            let path = folder + "/" + name + ".app"
            if FileManager.default.fileExists(atPath: path) {
                return workspace.icon(forFile: path)
            }
        }

        return nil
    }
}

// Which of the players Settings knows about are on this Mac, so it only
// offers the ones that are. Asked fresh each time Settings opens, so an app
// installed in the meantime shows up.
@MainActor
enum InstalledApps {
    // The short name in the settings file, and the app's bundle ID and name.
    private static let players: [String: (id: String, name: String)] = [
        "cider": ("sh.cider.genten.mac", "Cider"),
        "apple-music": ("com.apple.Music", "Music"),
        "spotify": ("com.spotify.client", "Spotify"),
        "firefox": ("org.mozilla.firefox", "Firefox"),
        "safari": ("com.apple.Safari", "Safari"),
        "chrome": ("com.google.Chrome", "Google Chrome"),
    ]

    // True for anything not in the list above: a player added to the
    // settings file by hand, which there is no telling about.
    static func has(_ key: String) -> Bool {
        guard let app = players[key] else { return true }

        if NSWorkspace.shared.urlForApplication(withBundleIdentifier: app.id) != nil {
            return true
        }

        // Cider, without knowing its bundle ID for sure: by name.
        let folders = ["/Applications", NSHomeDirectory() + "/Applications"]
        return folders.contains { FileManager.default.fileExists(atPath: $0 + "/" + app.name + ".app") }
    }
}

// Covers and other pictures, by address. Each is downloaded once; one that
// would not load is not tried again.
@MainActor
@Observable
final class Pictures {
    static let shared = Pictures()

    private(set) var loaded: [URL: NSImage] = [:]
    @ObservationIgnored private var asked: Set<URL> = []

    func image(_ url: URL?) -> NSImage? {
        guard let url else { return nil }

        if let image = loaded[url] { return image }

        if !asked.contains(url) {
            asked.insert(url)
            Task { await load(url) }
        }

        return nil
    }

    private func load(_ url: URL) async {
        guard let (data, _) = try? await URLSession.shared.data(from: url),
              let image = NSImage(data: data)
        else { return }

        loaded[url] = image
    }
}
