// What the window says about the status. The same words and rules as
// ui/app.js in the Tauri app and qt/ui/Texts.cpp in the Qt one, so every
// version says the same thing.

import Foundation

enum Texts {
    // Discord's activity types, which the site and the engine use too.
    static let playing = 0
    static let listening = 2

    // The apps that can be switched on in Settings, by the short name the
    // settings file uses for them. Anything else seen playing is added too.
    static let knownPlayers: [(key: String, name: String)] = [
        ("cider", "Cider"),
        ("apple-music", "Music"),
        ("spotify", "Spotify"),
        ("firefox", "Firefox"),
        ("safari", "Safari"),
        ("chrome", "Chrome"),
    ]

    // The music, and the first thing that is not music, as the site picks
    // them for its two cards.
    static func music(of status: Status) -> Activity? {
        status.activities.first { $0.isMusic }
    }

    static func other(of status: Status) -> Activity? {
        status.activities.first { !$0.isMusic }
    }

    // "Listening to", "Playing", "Watching"...
    static func heading(_ type: Int) -> String {
        switch type {
        case 1: "Streaming"
        case 2: "Listening to"
        case 3: "Watching"
        case 5: "Competing in"
        default: "Playing"
        }
    }

    // 02:11, or 01:02:11 past an hour - clock() in the site's lanyard.js.
    static func clock(_ milliseconds: Double) -> String {
        let total = max(0, Int(milliseconds / 1000))

        let hours = total / 3600
        let minutes = (total % 3600) / 60
        let seconds = total % 60

        var parts: [Int] = []
        if hours > 0 { parts.append(hours) }
        parts.append(minutes)
        parts.append(seconds)

        return parts.map { String(format: "%02d", $0) }.joined(separator: ":")
    }

    // Whether this computer is on the site.
    enum Sharing {
        case notSetUp, paused, unreachable, sharing
    }

    static func sharing(_ status: Status) -> Sharing {
        if !status.set_up { return .notSetUp }
        if status.paused { return .paused }
        if !status.server_ok { return .unreachable }
        return .sharing
    }

    static func sharingText(_ status: Status) -> String {
        switch sharing(status) {
        case .notSetUp: "Not set up yet. Add the server token in Settings."
        case .paused: "Paused. Nothing from this computer is on aridan.net."
        case .unreachable: "Can't reach the server: " + status.server_message
        case .sharing: "Sharing to aridan.net as " + status.device
        }
    }

    // The first line of the menu bar item's menu.
    static func menuLine(_ status: Status) -> String {
        if !status.set_up { return "Not set up yet" }

        guard let first = status.activities.first else {
            return status.paused ? "Paused" : "Nothing playing"
        }

        var line = first.details ?? first.name
        if let state = first.state, !state.isEmpty {
            line += " – " + state
        }

        return status.paused ? "Paused: " + line : line
    }

    // One app that may or may not be shared, for Settings.
    struct Player: Identifiable {
        var key: String
        var name: String
        var id: String { key }
    }

    // Kept in order: the known ones, then anything allowed, then anything
    // open. That is also the order they win in when two play at once.
    static func players(config: Config, status: Status) -> [Player] {
        var list = knownPlayers.map { Player(key: $0.key, name: $0.name) }

        func known(_ key: String) -> Bool {
            list.contains { $0.key == key }
        }

        for key in config.allowed_players where !known(key) {
            list.append(Player(key: key, name: key))
        }

        for player in status.players where !known(player.key) {
            list.append(Player(key: player.key, name: player.name))
        }

        return list
    }

    // How one source is doing, for Settings' Developer tab: "Active" when
    // something being shared right now came from it, "Idle" when nothing
    // did, or a problem that stops it working.
    struct Source: Identifiable {
        enum State { case active, idle, problem }

        var name: String
        var state: State
        var badge: String
        var users: [Activity]
        var id: String { name }
    }

    static func sources(_ status: Status) -> [Source] {
        func source(_ name: String, _ key: String, _ problem: String) -> Source {
            let users = status.activities.filter { $0.source == key }

            if !users.isEmpty { return Source(name: name, state: .active, badge: "Active", users: users) }
            if !problem.isEmpty { return Source(name: name, state: .problem, badge: problem, users: users) }
            return Source(name: name, state: .idle, badge: "Idle", users: users)
        }

        return [
            source("Cider", "cider", status.cider == "token-refused" ? "Token refused" : ""),
            source("Discord rich presence", "discord", status.discord == "unavailable" ? "Unavailable" : ""),
            source("Now Playing", "nowplaying", ""),
        ]
    }

    // Apple's covers can be asked for at any size, by changing the size in
    // the address - coverUrl() in ui/app.js.
    static func cover(_ address: String?, size: Int) -> URL? {
        guard let address, !address.isEmpty else { return nil }
        guard address.contains("mzstatic.com/") else { return URL(string: address) }

        let pattern = #"/(\d+)x\1(bb|sr)(-\d+)?\.(jpg|png)(\?.*)?$"#
        let resized = address.replacingOccurrences(
            of: pattern,
            with: "/\(size)x\(size)$2$3.$4$5",
            options: [.regularExpression, .caseInsensitive]
        )
        return URL(string: resized)
    }
}

// How far into an activity, and how long it is, in milliseconds.
struct Times {
    var elapsed: Double?
    var length: Double?
    var remaining: Double?

    init(_ activity: Activity?, now: Date) {
        let nowMs = now.timeIntervalSince1970 * 1000
        let start = activity?.timestamps?.start
        let end = activity?.timestamps?.end

        if let start, let end, end > start {
            length = end - start
            elapsed = min(max(0, nowMs - start), end - start)
        } else if let end {
            remaining = max(0, end - nowMs)
        } else if let start {
            elapsed = max(0, nowMs - start)
        }
    }

    var fraction: Double? {
        guard let elapsed, let length, length > 0 else { return nil }
        return elapsed / length
    }
}
