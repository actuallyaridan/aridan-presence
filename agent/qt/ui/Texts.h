#pragma once

// What the window says about the status, and small helpers the window, the
// tray and Settings share. The same words and rules as ui/app.js in the
// Tauri app, so every version says the same thing.

#include <QIcon>
#include <QJsonObject>
#include <QList>
#include <QString>

namespace Texts {

// Discord's activity types, which the site and the engine use too.
constexpr int Playing = 0;
constexpr int Listening = 2;

// The music, and the first thing that is not music, as the site picks
// them for its two cards. Empty when there is none.
QJsonObject musicOf(const QJsonObject &status);
QJsonObject otherOf(const QJsonObject &status);

// "Listening to", "Playing", "Watching"... for a card's title.
QString heading(int type);

// The app's icon from the icon theme, found by the name it shares under,
// or a plain one for music or for anything else.
QIcon iconFor(const QString &name, bool music);

// 02:11, or 1:02:11 past an hour - clock() in the site's lanyard.js.
QString clock(qint64 milliseconds);

// Whether this computer is on the site, and an icon to match.
struct Sharing {
    QString text;
    QIcon icon;
};
Sharing sharing(const QJsonObject &status);

// The first line of the tray menu, and its tooltip.
QString trayLine(const QJsonObject &status);

// One app that may or may not be shared, for Settings.
struct Player {
    QString key;
    QString name;
    bool allowed;
};
QList<Player> playerList(const QJsonObject &config, const QJsonObject &status);

// How one source is doing, for Settings' Developer tab: "Active" when
// something being shared right now came from it, "Idle" when nothing did,
// or a problem that stops it working. `state` is "current" (active),
// "unknown" (a problem) or "pending" (idle). `users` are the activities
// using it, for their icons.
struct Source {
    QString name;
    QString state;
    QString badge;
    QList<QJsonObject> users;
};
QList<Source> sources(const QJsonObject &status);

} // namespace Texts
