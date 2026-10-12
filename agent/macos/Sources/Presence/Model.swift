// What the Rust side sends over, as JSON - see core/src/state.rs,
// core/src/activity.rs and core/src/config.rs for what each field means.

import Foundation

struct Status: Decodable, Equatable {
    var set_up = false
    var paused = false
    var device = ""
    var server_ok = true
    var server_message = ""
    var cider = ""
    var discord = ""
    var players: [PlayerSeen] = []
    var activities: [Activity] = []
    var config_error = ""

    static let starting = Status()
}

struct PlayerSeen: Decodable, Equatable, Hashable {
    var key: String
    var name: String
    var allowed: Bool
    var playing: Bool
}

struct Activity: Decodable, Equatable {
    var type: Int
    var name: String
    var source: String
    var details: String?
    var state: String?
    var url: String?
    var timestamps: Timestamps?
    var assets: Assets?

    var isMusic: Bool { type == Texts.listening }
}

// Milliseconds since 1970, as JavaScript's Date.now() counts.
struct Timestamps: Decodable, Equatable {
    var start: Double?
    var end: Double?
}

struct Assets: Decodable, Equatable {
    var large_image: String?
    var large_text: String?
    var small_image: String?
    var small_text: String?
}

struct Config: Codable, Equatable {
    var server = "https://presence.aridan.net"
    var token = ""
    var device = ""
    var cider_token = ""
    var itunes_countries: [String] = []
    var allowed_players: [String] = []
}

struct LogLine: Decodable, Identifiable {
    var at: Double
    var text: String

    var id: String { "\(at)\(text)" }
}
