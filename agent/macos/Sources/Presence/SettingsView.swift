// Settings, as a Mac app has them: a window of its own with a tab for each
// part across its toolbar, opened from the app menu with Command-comma.
//
//   General       start at login, and which apps may be shared
//   Connections   the server, Cider, and where covers are looked up
//   Developer     how each source is doing, and the log
//
// As in the rest of macOS, there is nothing to Save: a switch takes effect
// as soon as it is flipped, and a text field as soon as it is left.

import AppKit
import ServiceManagement
import SwiftUI

struct SettingsView: View {
    enum Page: Hashable {
        case general, connections, developer
    }

    @State private var editor = SettingsEditor()

    // Not set up yet: Connections is where the token goes.
    @State private var page: Page = Presence.shared.status.set_up ? .general : .connections

    var body: some View {
        TabView(selection: $page) {
            Tab("General", systemImage: "gearshape", value: .general) {
                GeneralSettings(editor: editor)
            }

            Tab("Connections", systemImage: "network", value: .connections) {
                ConnectionSettings(editor: editor)
            }

            Tab("Developer", systemImage: "hammer", value: .developer) {
                DeveloperSettings()
            }
        }
        .frame(width: 520)
        .background(PlainTitleBar())
        .onAppear {
            // Opened from the menu bar, the app may not be in front yet.
            NSApp.activate()
        }
    }
}

// macOS 26 draws the title bar's background, with a line under it, only
// while the pointer is over it - so the top of Settings flickered every time
// the pointer passed. Made see-through for good, it always looks as it does
// with the pointer elsewhere. SwiftUI has no say over the Settings window's
// title bar, so this reaches the window through AppKit.
private struct PlainTitleBar: NSViewRepresentable {
    func makeNSView(context: Context) -> NSView {
        WindowWatcher()
    }

    func updateNSView(_ view: NSView, context: Context) {}

    private final class WindowWatcher: NSView {
        override func viewDidMoveToWindow() {
            super.viewDidMoveToWindow()
            guard let window else { return }

            window.titlebarAppearsTransparent = true
            window.titlebarSeparatorStyle = .none
        }
    }
}

// The settings being edited, shared by the tabs, and saving them.
@MainActor
@Observable
final class SettingsEditor {
    var config: Config
    var countries: String

    // A problem with the settings file at startup, or with saving.
    var problem: String

    private var saved: Config

    init() {
        let config = Presence.shared.config()

        self.config = config
        self.saved = config
        self.countries = config.itunes_countries.joined(separator: ", ")
        self.problem = Presence.shared.status.config_error
    }

    func save() {
        config.itunes_countries = countries
            .split(separator: ",")
            .map { $0.trimmingCharacters(in: .whitespaces) }
            .filter { !$0.isEmpty }

        guard config != saved else { return }

        if let problem = Presence.shared.save(config) {
            self.problem = problem
            return
        }

        // As written to the file: tidied up by the Rust side.
        let tidied = Presence.shared.config()
        saved = tidied
        config = tidied
        countries = tidied.itunes_countries.joined(separator: ", ")
        problem = ""
    }

    func allowed(_ key: String) -> Binding<Bool> {
        Binding {
            self.config.allowed_players.contains(key)
        } set: { allowed in
            self.config.allowed_players.removeAll { $0 == key }
            if allowed {
                self.config.allowed_players.append(key)
            }
            self.save()
        }
    }
}

// A problem, in red, at the top of the tab it matters to.
private struct ProblemSection: View {
    var problem: String

    var body: some View {
        if !problem.isEmpty {
            Section {
                Label {
                    Text(problem)
                        .textSelection(.enabled)
                } icon: {
                    Image(systemName: "exclamationmark.triangle.fill")
                }
                .foregroundStyle(.red)
            }
        }
    }
}

// MARK: - General

private struct GeneralSettings: View {
    var editor: SettingsEditor

    @State private var login = LoginItem()

    // The known players on this Mac, looked up when the tab opens.
    @State private var installed: Set<String> = []

    var body: some View {
        let status = Presence.shared.status

        Form {
            ProblemSection(problem: editor.problem)

            Section {
                Toggle("Start when I log in", isOn: $login.enabled)
            } footer: {
                if let note = login.note {
                    HStack {
                        Text(note)
                        if login.needsApproval {
                            Button("Open Login Items…") {
                                SMAppService.openSystemSettingsLoginItems()
                            }
                            .buttonStyle(.link)
                        }
                    }
                    .foregroundStyle(.secondary)
                }
            }

            Section {
                ForEach(installedPlayers(status)) { player in
                    Toggle(isOn: editor.allowed(player.key)) {
                        Label {
                            Text(player.name)
                        } icon: {
                            AppIcon(name: player.name, music: true, size: 20)
                        }
                    }
                }
            } header: {
                Text("Apps that may be shared")
            } footer: {
                Text("Anything else playing – a video call, a training video – is never shared.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .onAppear {
            login.refresh()
            installed = Set(Texts.knownPlayers.map(\.key).filter(InstalledApps.has))
        }
        .frame(height: 470)
    }
}

extension GeneralSettings {
    // Only the apps on this Mac. One that is not installed but is playing
    // anyway - which cannot really happen - or was added to the settings
    // file by hand is still shown. Hidden ones keep their setting.
    fileprivate func installedPlayers(_ status: Status) -> [Texts.Player] {
        let known = Set(Texts.knownPlayers.map(\.key))
        let playing = Set(status.players.map(\.key))

        return Texts.players(config: editor.config, status: status).filter { player in
            !known.contains(player.key) || installed.contains(player.key) || playing.contains(player.key)
        }
    }
}

// Start at login, the way macOS 13 and later do it: the app itself is the
// login item, and it shows up in System Settings › General › Login Items.
@MainActor
@Observable
final class LoginItem {
    private(set) var note: String?
    private(set) var needsApproval = false
    private var on = false

    init() { refresh() }

    func refresh() {
        let status = SMAppService.mainApp.status
        on = status == .enabled || status == .requiresApproval
        needsApproval = status == .requiresApproval
        note = needsApproval ? "Allow aridan-presence in Login Items for this to work." : nil
    }

    var enabled: Bool {
        get { on }
        set {
            do {
                if newValue {
                    try SMAppService.mainApp.register()
                } else {
                    try SMAppService.mainApp.unregister()
                }
                refresh()
            } catch {
                refresh()
                note = "Could not change start at login: " + error.localizedDescription
            }
        }
    }
}

// MARK: - Connections

private struct ConnectionSettings: View {
    @Bindable var editor: SettingsEditor

    private enum Field: Hashable {
        case token, server, device, ciderToken, countries
    }

    @FocusState private var focus: Field?

    var body: some View {
        Form {
            ProblemSection(problem: editor.problem)

            Section {
                SecureField("Token", text: $editor.config.token)
                    .focused($focus, equals: .token)

                TextField("Address", text: $editor.config.server)
                    .focused($focus, equals: .server)

                TextField("Computer name", text: $editor.config.device, prompt: Text(Presence.shared.status.device))
                    .focused($focus, equals: .device)
            } header: {
                Text("Server")
            } footer: {
                Text("The token is the AGENT_TOKEN the Worker was given. Leave the computer name empty to use one made from this Mac's name.")
                    .foregroundStyle(.secondary)
            }

            Section {
                SecureField("API token", text: $editor.config.cider_token, prompt: Text("Optional"))
                    .focused($focus, equals: .ciderToken)
            } header: {
                Text("Cider")
            } footer: {
                Text("From Cider › Settings › Connectivity. Without it, Cider is read through Discord.")
                    .foregroundStyle(.secondary)
            }

            Section {
                TextField("iTunes stores", text: $editor.countries, prompt: Text("se, us"))
                    .focused($focus, equals: .countries)
            } header: {
                Text("Artwork")
            } footer: {
                Text("Searched in this order, separated by commas.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
        .autocorrectionDisabled()
        .onSubmit { editor.save() }
        .onChange(of: focus) { editor.save() }
        .onDisappear { editor.save() }
        .frame(height: 470)
    }
}

// MARK: - Developer

private struct DeveloperSettings: View {
    @State private var log: [LogLine] = []

    var body: some View {
        let presence = Presence.shared

        Form {
            Section("Sources") {
                ForEach(Texts.sources(presence.status)) { source in
                    SourceRow(source: source)
                }
            }

            Section("Log") {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 3) {
                        ForEach(Array(log.reversed().enumerated()), id: \.offset) { _, line in
                            HStack(alignment: .firstTextBaseline, spacing: 10) {
                                Text(Date(timeIntervalSince1970: line.at / 1000), format: .dateTime.hour(.twoDigits(amPM: .omitted)).minute().second())
                                    .foregroundStyle(.secondary)
                                Text(line.text)
                            }
                        }
                    }
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                .frame(height: 150)
            }

            Section {
                LabeledContent("Version", value: presence.version)

                LabeledContent("Settings file") {
                    HStack {
                        Text(presence.configPath)
                            .lineLimit(1)
                            .truncationMode(.head)
                            .textSelection(.enabled)
                            .help(presence.configPath)
                        Button("Show in Finder") {
                            let url = URL(fileURLWithPath: presence.configPath)
                            if FileManager.default.fileExists(atPath: url.path) {
                                NSWorkspace.shared.activateFileViewerSelecting([url])
                            } else {
                                NSWorkspace.shared.open(url.deletingLastPathComponent())
                            }
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
        .frame(height: 470)
        .task {
            // The log follows along while the tab is open.
            while !Task.isCancelled {
                log = presence.log()
                try? await Task.sleep(for: .seconds(1))
            }
        }
    }
}

// Name on the left and whether it is active on the right, with the icons of
// whatever is using it under the name.
private struct SourceRow: View {
    var source: Texts.Source

    var body: some View {
        LabeledContent {
            Text(source.badge)
                .foregroundStyle(color)
        } label: {
            Text(source.name)

            if !source.users.isEmpty {
                HStack(spacing: 6) {
                    ForEach(Array(source.users.enumerated()), id: \.offset) { _, activity in
                        AppIcon(name: activity.name, music: activity.isMusic, size: 16)
                            .help(activity.name)
                    }
                }
            }
        }
    }

    private var color: Color {
        switch source.state {
        case .active: .green
        case .problem: .red
        case .idle: .secondary
        }
    }
}
