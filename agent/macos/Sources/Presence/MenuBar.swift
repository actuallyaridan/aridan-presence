// The item in the menu bar, and its menu:
//
//   Popila – Severina          what is being shared (not clickable)
//   ─────────────
//   ✓ Pause Sharing
//   Open aridan-presence
//   Settings…
//   ─────────────
//   Quit aridan-presence
//
// Its icon is the app's note, drawn the way menu bar icons are - in one
// colour that macOS tints to suit the menu bar - and faded while sharing is
// paused or nothing is set up yet.

import AppKit
import SwiftUI

struct MenuBarMenu: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        let presence = Presence.shared
        let status = presence.status

        Text(Texts.menuLine(status))

        Divider()

        Toggle("Pause Sharing", isOn: Binding {
            status.paused
        } set: {
            presence.setPaused($0)
        })
        .disabled(!status.set_up)

        Button("Open aridan-presence") {
            presence.openWindowAction = openWindow
            presence.requestShow()
        }

        SettingsLink {
            Text("Settings…")
        }
        .keyboardShortcut(",")

        Divider()

        Button("Quit aridan-presence") {
            NSApp.terminate(nil)
        }
        .keyboardShortcut("q")
    }
}

// Always there, so it is also what opens the window when AppKit's side asks
// for it before any window has been open - see Presence.requestShow.
struct MenuBarLabel: View {
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        let presence = Presence.shared
        let status = presence.status
        let sharing = status.set_up && !status.paused

        Image(nsImage: MenuBarIcon.image(faded: !sharing))
            .onChange(of: presence.showPending, initial: true) {
                presence.openWindowAction = presence.openWindowAction ?? openWindow

                if presence.showPending {
                    presence.showPending = false
                    presence.requestShow()
                }
            }
    }
}

// The note and the two waves from icons/source.svg, without the tile behind
// them, at menu bar size.
enum MenuBarIcon {
    private static let on = draw(faded: false)
    private static let off = draw(faded: true)

    static func image(faded: Bool) -> NSImage {
        faded ? off : on
    }

    private static func draw(faded: Bool) -> NSImage {
        let size = NSSize(width: 20, height: 18)

        let image = NSImage(size: size, flipped: true) { _ in
            // source.svg's units: the drawing spans about 262...820 across
            // and 250...800 down, out of 1024.
            let scale = 16.5 / 560.0
            var transform = AffineTransform(translationByX: (20 - 558 * scale) / 2, byY: (18 - 550 * scale) / 2)
            transform.scale(scale)
            transform.translate(x: -262, y: -250)

            let alpha = faded ? 0.4 : 1.0

            // The note: its head, stem and flag.
            let note = NSBezierPath()

            let head = NSBezierPath(ovalIn: NSRect(x: 380 - 118, y: 700 - 94, width: 236, height: 188))
            var tilt = AffineTransform(translationByX: 380, byY: 700)
            tilt.rotate(byDegrees: -20)
            tilt.translate(x: -380, y: -700)
            head.transform(using: tilt)
            note.append(head)

            note.append(NSBezierPath(roundedRect: NSRect(x: 462, y: 250, width: 56, height: 460), xRadius: 20, yRadius: 20))

            let flag = NSBezierPath()
            flag.move(to: NSPoint(x: 490, y: 250))
            flag.curve(to: NSPoint(x: 620, y: 430), controlPoint1: NSPoint(x: 600, y: 270), controlPoint2: NSPoint(x: 640, y: 340))
            flag.curve(to: NSPoint(x: 500, y: 345), controlPoint1: NSPoint(x: 600, y: 380), controlPoint2: NSPoint(x: 560, y: 350))
            flag.close()
            note.append(flag)

            note.transform(using: transform)
            NSColor.black.withAlphaComponent(alpha).setFill()
            note.fill()

            // The waves: it is being sent somewhere.
            let waves: [(from: NSPoint, to: NSPoint, radius: CGFloat, opacity: CGFloat)] = [
                (NSPoint(x: 660, y: 520), NSPoint(x: 660, y: 670), 110, 0.85),
                (NSPoint(x: 740, y: 455), NSPoint(x: 740, y: 735), 200, 0.55),
            ]

            for wave in waves {
                let path = arc(from: wave.from, to: wave.to, radius: wave.radius)
                path.transform(using: transform)
                path.lineWidth = 44 * scale
                path.lineCapStyle = .round
                NSColor.black.withAlphaComponent(alpha * wave.opacity).setStroke()
                path.stroke()
            }

            return true
        }

        image.isTemplate = true
        image.accessibilityDescription = "aridan-presence"
        return image
    }

    // An arc bulging to the right between two points straight above one
    // another, as the SVG's "a r r 0 0 1" draws it.
    private static func arc(from start: NSPoint, to end: NSPoint, radius: CGFloat) -> NSBezierPath {
        let half = (end.y - start.y) / 2
        let center = NSPoint(x: start.x - (radius * radius - half * half).squareRoot(), y: start.y + half)

        let first = atan2(start.y - center.y, start.x - center.x)
        let last = atan2(end.y - center.y, end.x - center.x)

        let path = NSBezierPath()
        let steps = 24
        for step in 0...steps {
            let angle = first + (last - first) * CGFloat(step) / CGFloat(steps)
            let point = NSPoint(x: center.x + radius * cos(angle), y: center.y + radius * sin(angle))
            if step == 0 { path.move(to: point) } else { path.line(to: point) }
        }
        return path
    }
}
