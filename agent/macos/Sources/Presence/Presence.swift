// The Swift half of the bridge to Rust (src/lib.rs), and what the window,
// the menu bar item and Settings all watch.
//
// Rust calls back from its own threads; everything here happens on the main
// thread, so the callbacks hop over to it before touching anything.

import AppKit
import CPresence
import Observation
import SwiftUI

@MainActor
@Observable
final class Presence {
    static let shared = Presence()

    // The engine's latest status.
    private(set) var status = Status.starting

    // Something asked for the window - the menu bar, the Dock, a second copy
    // of the app being opened - before there was a way to open it. The menu
    // bar item watches this and opens it - see MenuBarLabel.
    var showPending = false

    // SwiftUI's way to open the window, kept from the first view that had
    // it, so AppKit's side can open the window too.
    @ObservationIgnored var openWindowAction: OpenWindowAction?

    let version: String
    let configPath: String

    private init() {
        presence_start(onStatus)

        version = takeString(presence_version())
        configPath = takeString(presence_config_path())
        status = decode(Status.self, from: takeString(presence_status_json())) ?? .starting
    }

    // The settings as saved, read fresh from Rust.
    func config() -> Config {
        decode(Config.self, from: takeString(presence_config_json())) ?? Config()
    }

    // Saves the settings. Gives back nil if that worked, or why not.
    func save(_ config: Config) -> String? {
        guard let data = try? JSONEncoder().encode(config),
              let json = String(data: data, encoding: .utf8)
        else {
            return "Could not write the settings."
        }

        let problem = json.withCString { takeString(presence_save_config($0)) }
        return problem.isEmpty ? nil : problem
    }

    func log() -> [LogLine] {
        decode([LogLine].self, from: takeString(presence_log_json())) ?? []
    }

    func setPaused(_ paused: Bool) {
        presence_set_paused(paused)

        // Shown straight away, rather than at the engine's next status.
        status.paused = paused
    }

    func requestShow() {
        guard let openWindowAction else {
            showPending = true
            return
        }

        openWindowAction(id: "main")
        NSApp.activate()
    }

    // Takes this computer off the site, then calls `done`.
    func clearFromSite(then done: @escaping @MainActor () -> Void) {
        Presence.whenCleared = done
        presence_quit(onCleared)
    }

    fileprivate static var whenCleared: (@MainActor () -> Void)?

    fileprivate func receive(_ json: String) {
        guard let new = decode(Status.self, from: json), new != status else { return }
        status = new
    }
}

// Called by Rust, from one of its threads. C function pointers cannot hold
// on to anything, so these are free functions that find Presence.shared
// once they are on the main thread.
private let onStatus: @convention(c) (UnsafeMutablePointer<CChar>?) -> Void = { pointer in
    let json = takeString(pointer)

    DispatchQueue.main.async {
        MainActor.assumeIsolated {
            Presence.shared.receive(json)
        }
    }
}

private let onCleared: @convention(c) () -> Void = {
    DispatchQueue.main.async {
        MainActor.assumeIsolated {
            let done = Presence.whenCleared
            Presence.whenCleared = nil
            done?()
        }
    }
}

// Copies a string Rust made, then gives it back to Rust to free.
private func takeString(_ pointer: UnsafeMutablePointer<CChar>?) -> String {
    guard let pointer else { return "" }
    let text = String(cString: pointer)
    presence_free_string(pointer)
    return text
}

private func decode<T: Decodable>(_ type: T.Type, from json: String) -> T? {
    try? JSONDecoder().decode(type, from: Data(json.utf8))
}
