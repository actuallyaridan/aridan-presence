// The window: what is being shared - the cover, with the song beside it -
// and, under the window's title, whether this computer is on the site.
//
// It is an ordinary window with the system's own background, light or dark
// as macOS is. The pause and Settings buttons are in the toolbar, which
// macOS draws in glass.

import AppKit
import SwiftUI

struct NowPlayingWindow: View {
    @Environment(\.openSettings) private var openSettings
    @Environment(\.openWindow) private var openWindow

    private var presence = Presence.shared
    private var pictures = Pictures.shared

    var body: some View {
        let status = presence.status
        let music = Texts.music(of: status)
        let other = Texts.other(of: status)

        // The music takes the middle; or with none, whatever else there is.
        let main = music ?? other
        let corner = music == nil ? nil : other

        let cover = pictures.image(Texts.cover(main?.assets?.large_image, size: 600))

        VStack(spacing: 0) {
            Group {
                if !status.set_up {
                    SetUpPrompt()
                } else if let main {
                    ActivityView(activity: main, cover: cover)
                } else {
                    NothingPlaying()
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)

            if let corner {
                CornerPill(activity: corner)
                    .frame(maxWidth: .infinity, alignment: .trailing)
                    .padding(16)
            }
        }
        .frame(width: 700, height: 420)
        // "Sharing to aridan.net as mac", under the title, as Mail puts how
        // many messages a mailbox has.
        .navigationSubtitle(Texts.sharingText(status))
        .toolbar {
            ToolbarItemGroup(placement: .primaryAction) {
                Button {
                    presence.setPaused(!status.paused)
                } label: {
                    if status.paused {
                        Label("Resume Sharing", systemImage: "play.fill")
                    } else {
                        Label("Pause Sharing", systemImage: "pause.fill")
                    }
                }
                .help(status.paused ? "Resume sharing" : "Pause sharing")
                .disabled(!status.set_up)

                SettingsLink {
                    Label("Settings", systemImage: "gearshape")
                }
                .help("Settings")
            }
        }
        .onAppear {
            Presence.shared.openWindowAction = openWindow

            // In the Dock and the app switcher while the window is open; only
            // in the menu bar the rest of the time.
            NSApp.setActivationPolicy(.regular)
            NSApp.activate()

            // Nothing works until there is a token, and that is set in
            // Settings, so a first start opens it.
            if !status.set_up {
                openSettings()
            }
        }
        .onDisappear {
            NSApp.setActivationPolicy(.accessory)
        }
    }
}

// The cover or the app's icon on the left, and what it is on the right.
private struct ActivityView: View {
    var activity: Activity
    var cover: NSImage?

    var body: some View {
        HStack(spacing: 32) {
            Artwork(activity: activity, cover: cover)
            Details(activity: activity)
        }
        .padding(.horizontal, 44)
    }
}

private struct Artwork: View {
    var activity: Activity
    var cover: NSImage?

    private let shape = RoundedRectangle(cornerRadius: 16, style: .continuous)

    var body: some View {
        Group {
            if let cover {
                Image(nsImage: cover)
                    .resizable()
                    .aspectRatio(contentMode: .fill)
                    .clipShape(shape)
                    .overlay(shape.strokeBorder(.separator, lineWidth: 1))
                    .shadow(color: .black.opacity(0.2), radius: 12, y: 6)
            } else {
                // A pane of glass with the app's icon on it, until there is a
                // cover - or for good, for an app that has none.
                AppIcon(name: activity.name, music: activity.isMusic, size: 96)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .glassEffect(.regular, in: shape)
            }
        }
        .frame(width: 210, height: 210)
    }
}

private struct Details: View {
    var activity: Activity

    var body: some View {
        // A song's name is its title; a game's own name is, with what is
        // going on in it under that.
        let title = activity.isMusic ? (activity.details ?? activity.name) : activity.name
        let subtitle = activity.isMusic ? activity.state : activity.details
        let third = activity.isMusic ? activity.assets?.large_text : activity.state

        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 6) {
                AppIcon(name: activity.name, music: activity.isMusic, size: 16)
                Text(Texts.heading(activity.type) + " " + activity.name)
                    .lineLimit(1)
            }
            .font(.subheadline.weight(.medium))
            .foregroundStyle(.secondary)
            .padding(.bottom, 10)

            Text(title)
                .font(.title.weight(.bold))
                .lineLimit(2)
                .textSelection(.enabled)

            if let subtitle, !subtitle.isEmpty {
                Text(subtitle)
                    .font(.title3)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .padding(.top, 4)
            }

            if let third, !third.isEmpty {
                Text(third)
                    .font(.body)
                    .foregroundStyle(.tertiary)
                    .lineLimit(1)
                    .padding(.top, 2)
            }

            Progress(activity: activity)
                .padding(.top, 22)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

// The line and the clocks under it, moving every second between status
// updates. Without both a start and an end there is no line, only a clock.
private struct Progress: View {
    var activity: Activity

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            let times = Times(activity, now: context.date)

            VStack(spacing: 6) {
                if let fraction = times.fraction {
                    GeometryReader { geometry in
                        ZStack(alignment: .leading) {
                            Capsule().fill(.quaternary)
                            Capsule()
                                .fill(.secondary)
                                .frame(width: max(5, geometry.size.width * fraction))
                        }
                    }
                    .frame(height: 5)
                    .animation(.linear(duration: 1), value: fraction)
                }

                HStack {
                    if let elapsed = times.elapsed {
                        Text(Texts.clock(elapsed))
                    }
                    Spacer()
                    if let length = times.length, let elapsed = times.elapsed {
                        Text("-" + Texts.clock(length - elapsed))
                    } else if let remaining = times.remaining {
                        Text(Texts.clock(remaining) + " left")
                    }
                }
                .font(.caption.monospacedDigit())
                .foregroundStyle(.secondary)
            }
        }
        .frame(maxWidth: 360)
    }
}

private struct NothingPlaying: View {
    var body: some View {
        VStack(spacing: 16) {
            Image(systemName: "music.note")
                .font(.system(size: 40, weight: .medium))
                .foregroundStyle(.secondary)
                .frame(width: 96, height: 96)
                .glassEffect(.regular, in: .circle)

            Text("Nothing is playing")
                .font(.title3.weight(.medium))
                .foregroundStyle(.secondary)
        }
    }
}

private struct SetUpPrompt: View {
    @Environment(\.openSettings) private var openSettings

    var body: some View {
        VStack(spacing: 12) {
            Image(nsImage: NSApp.applicationIconImage)
                .resizable()
                .frame(width: 96, height: 96)

            Text("Set up aridan-presence")
                .font(.title2.weight(.bold))

            Text("Add the server token in Settings to start sharing\nwhat you're listening to and playing.")
                .multilineTextAlignment(.center)
                .foregroundStyle(.secondary)

            Button("Open Settings…") {
                openSettings()
            }
            .buttonStyle(.glassProminent)
            .controlSize(.large)
            .padding(.top, 6)
        }
    }
}

// Anything else going on - a game alongside the music - small, in the
// bottom corner: its icon, what it is, and its clock.
private struct CornerPill: View {
    var activity: Activity

    var body: some View {
        TimelineView(.periodic(from: .now, by: 1)) { context in
            let times = Times(activity, now: context.date)

            HStack(spacing: 8) {
                AppIcon(name: activity.name, music: activity.isMusic, size: 20)

                VStack(alignment: .leading, spacing: 0) {
                    Text(activity.name)
                        .font(.callout.weight(.semibold))
                        .lineLimit(1)

                    if let line = line(times) {
                        Text(line)
                            .font(.caption.monospacedDigit())
                            .foregroundStyle(.secondary)
                            .lineLimit(1)
                    }
                }
            }
            .padding(.leading, 8)
            .padding(.trailing, 14)
            .padding(.vertical, 5)
            .glassEffect(.regular, in: .capsule)
            .frame(maxWidth: 260, alignment: .trailing)
        }
    }

    private func line(_ times: Times) -> String? {
        var parts: [String] = []
        if let details = activity.details, !details.isEmpty { parts.append(details) }

        if let remaining = times.remaining {
            parts.append(Texts.clock(remaining) + " left")
        } else if let elapsed = times.elapsed {
            parts.append(Texts.clock(elapsed))
        }

        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }
}

// The app's own icon if it is on this Mac, or a symbol for music or for
// anything else.
struct AppIcon: View {
    var name: String
    var music: Bool
    var size: CGFloat

    var body: some View {
        if let icon = AppIcons.icon(for: name) {
            Image(nsImage: icon)
                .resizable()
                .frame(width: size, height: size)
        } else {
            Image(systemName: music ? "music.note" : "app.dashed")
                .font(.system(size: size * 0.7, weight: .medium))
                .foregroundStyle(.secondary)
                .frame(width: size, height: size)
        }
    }
}
