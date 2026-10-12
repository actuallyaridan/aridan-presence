// aridan-presence for macOS: tells presence.aridan.net what I am listening to
// and playing, so the music widget on aridan.net works without Discord open.
//
// This is the Mac app, in SwiftUI, so it is drawn with Liquid Glass like the
// rest of macOS. Windows has the Tauri app (../src) and Linux the Qt one
// (../qt). All of them use ../core, which does the work, through a small
// bridge - here, src/lib.rs and Presence.swift.
//
// It lives in the menu bar. The window is only for looking at what it is
// doing; closing it leaves the app running there, out of the Dock. Quit from
// the menu bar item, or with Command-Q while the window is open.

import AppKit
import SwiftUI

@main
struct PresenceApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate

    init() {
        // Two copies would fight over discord-ipc-0 and report over each
        // other, so a second one shows the first one's window and stops -
        // before it starts anything of its own.
        SingleInstance.handOverIfRunning()
    }

    var body: some Scene {
        Window("aridan-presence", id: "main") {
            NowPlayingWindow()
        }
        .windowResizability(.contentSize)
        .defaultLaunchBehavior(.suppressed)
        .restorationBehavior(.disabled)
        .commands {
            CommandGroup(replacing: .newItem) {}

            CommandMenu("Sharing") {
                SharingCommands()
            }
        }

        Settings {
            SettingsView()
        }

        MenuBarExtra {
            MenuBarMenu()
        } label: {
            MenuBarLabel()
        }
    }
}

private struct SharingCommands: View {
    var body: some View {
        let presence = Presence.shared

        Toggle("Pause Sharing", isOn: Binding {
            presence.status.paused
        } set: {
            presence.setPaused($0)
        })
        .keyboardShortcut("p", modifiers: [.command, .shift])
        .disabled(!presence.status.set_up)
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private var quitting = false
    private var replied = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Starts the engine.
        let presence = Presence.shared

        SingleInstance.listen {
            Presence.shared.requestShow()
        }

        // Started at login, it stays in the menu bar - unless it is not set
        // up yet, since Settings is where that gets done.
        if !Self.startedAtLogin() || !presence.status.set_up {
            presence.requestShow()
        }
    }

    // Closing the window leaves the app in the menu bar.
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }

    // Clicking the app in the Dock or Finder while it runs opens the window.
    func applicationShouldHandleReopen(_ sender: NSApplication, hasVisibleWindows: Bool) -> Bool {
        if !hasVisibleWindows {
            Presence.shared.requestShow()
        }
        return true
    }

    // Quitting - from the menu, or logging out or shutting down - is the one
    // time the app goes away on purpose, so it is worth the moment it takes
    // to take this computer off the site, instead of leaving it there for the
    // Worker's 90 seconds.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if quitting { return .terminateLater }
        quitting = true

        Presence.shared.clearFromSite { [weak self] in
            self?.letQuit()
        }

        // The server gives up after ten seconds; this does too, a little after.
        DispatchQueue.main.asyncAfter(deadline: .now() + 12) { [weak self] in
            MainActor.assumeIsolated { self?.letQuit() }
        }

        return .terminateLater
    }

    private func letQuit() {
        guard !replied else { return }
        replied = true
        NSApp.reply(toApplicationShouldTerminate: true)
    }

    // Login items are opened with an Apple event that says so. Started from
    // the command line, --minimized does the same.
    private static func startedAtLogin() -> Bool {
        if CommandLine.arguments.contains("--minimized") { return true }

        guard let event = NSAppleEventManager.shared().currentAppleEvent else { return false }

        return event.eventID == kAEOpenApplication
            && event.paramDescriptor(forKeyword: keyAEPropData)?.enumCodeValue == keyAELaunchedAsLogInItem
    }
}

// Only one copy at a time, asked over the distributed notification centre,
// which every app on the Mac can post to.
enum SingleInstance {
    private static let show = Notification.Name("net.aridan.presence.show")

    static func handOverIfRunning() {
        guard let id = Bundle.main.bundleIdentifier else { return }

        let me = ProcessInfo.processInfo.processIdentifier
        let others = NSRunningApplication.runningApplications(withBundleIdentifier: id)
            .filter { $0.processIdentifier != me }

        guard let first = others.first else { return }

        DistributedNotificationCenter.default().postNotificationName(show, object: nil, deliverImmediately: true)
        first.activate()

        print("Already running. Showed its window instead.")
        exit(0)
    }

    static func listen(_ action: @escaping @MainActor () -> Void) {
        DistributedNotificationCenter.default().addObserver(forName: show, object: nil, queue: .main) { _ in
            MainActor.assumeIsolated { action() }
        }
    }
}
