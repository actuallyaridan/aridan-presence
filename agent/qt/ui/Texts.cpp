#include "Texts.h"
#include "IconHelper.h"

#include <QJsonArray>
#include <QMap>
#include <QStringList>

namespace Texts {

namespace {

// The apps that can be switched on in Settings, by the short name the
// settings file uses for them. Anything else seen playing is added too.
const QList<QPair<QString, QString>> kKnownPlayers = {
    {"cider", "Cider"},
    {"apple-music", "Apple Music app"},
    {"spotify", "Spotify"},
    {"firefox", "Firefox"},
    {"safari", "Safari"},
    {"chrome", "Chrome"},
};

QJsonArray activities(const QJsonObject &status) {
    return status.value("activities").toArray();
}

} // namespace

QJsonObject musicOf(const QJsonObject &status) {
    for (const QJsonValue &value : activities(status)) {
        const QJsonObject activity = value.toObject();
        if (activity.value("type").toInt() == Listening) return activity;
    }
    return QJsonObject();
}

QJsonObject otherOf(const QJsonObject &status) {
    for (const QJsonValue &value : activities(status)) {
        const QJsonObject activity = value.toObject();
        if (activity.value("type").toInt() != Listening) return activity;
    }
    return QJsonObject();
}

QString heading(int type) {
    switch (type) {
    case 1: return "Streaming";
    case 2: return "Listening to";
    case 3: return "Watching";
    case 5: return "Competing in";
    default: return "Playing";
    }
}

QIcon iconFor(const QString &name, bool music) {
    const QString lower = name.toLower();
    QIcon icon;

    // Cider and Apple Music have the plain music icon, as the site's note.
    // Others by the names their icons usually go by.
    if (lower.contains("apple music") || lower.contains("cider")) {
        icon = QIcon();
    } else if (lower.contains("spotify")) {
        icon = themeIcon({"spotify-client", "spotify", "com.spotify.Client"});
    } else if (lower.contains("firefox")) {
        icon = themeIcon({"firefox", "org.mozilla.firefox"});
    } else if (lower.contains("chrome")) {
        icon = themeIcon({"google-chrome", "chromium", "com.google.Chrome"});
    } else if (lower.contains("edge")) {
        icon = themeIcon({"microsoft-edge", "com.microsoft.Edge"});
    } else if (lower.contains("visual studio code")) {
        icon = themeIcon({"visual-studio-code", "code", "com.visualstudio.code"});
    } else if (!lower.isEmpty()) {
        // Most apps name their icon after themselves: "Steam" is "steam".
        QString guess = lower;
        guess.replace(' ', '-');
        icon = tryIconName(guess);
    }

    if (!icon.isNull()) return icon;

    if (music) {
        return themeIcon({"audio-x-generic", "multimedia-audio-player", "media-optical-audio"});
    }
    return themeIcon({"applications-other", "application-x-executable", "applications-system"});
}

QString clock(qint64 milliseconds) {
    const qint64 total = qMax<qint64>(0, milliseconds / 1000);

    const qint64 hours = total / 3600;
    const qint64 minutes = (total % 3600) / 60;
    const qint64 seconds = total % 60;

    QStringList parts;
    if (hours > 0) parts << QString::number(hours).rightJustified(2, '0');
    parts << QString::number(minutes).rightJustified(2, '0');
    parts << QString::number(seconds).rightJustified(2, '0');

    return parts.join(':');
}

Sharing sharing(const QJsonObject &status) {
    if (!status.value("set_up").toBool()) {
        return {"Not set up yet. Add the server token in Settings.",
                themeIcon({"dialog-warning", "emblem-important"})};
    }
    if (status.value("paused").toBool()) {
        return {"Paused. Nothing from this computer is on aridan.net.",
                themeIcon({"media-playback-pause", "media-playback-paused"})};
    }
    if (!status.value("server_ok").toBool()) {
        return {"Can't reach the server: " + status.value("server_message").toString(),
                themeIcon({"dialog-error", "emblem-error"})};
    }
    return {"Sharing to aridan.net as " + status.value("device").toString(),
            themeIcon({"emblem-ok", "emblem-default", "dialog-ok"})};
}

QString trayLine(const QJsonObject &status) {
    if (!status.value("set_up").toBool()) return "Not set up yet";

    const bool paused = status.value("paused").toBool();
    const QJsonArray list = activities(status);

    if (list.isEmpty()) {
        if (paused) return "Paused";
        return "Nothing playing";
    }

    const QJsonObject first = list.first().toObject();

    QString line = first.value("details").toString();
    if (line.isEmpty()) line = first.value("name").toString();

    const QString state = first.value("state").toString();
    if (!state.isEmpty()) line += " - " + state;

    if (paused) line = "Paused: " + line;
    return line;
}

QList<Player> playerList(const QJsonObject &config, const QJsonObject &status) {
    QStringList allowed;
    for (const QJsonValue &value : config.value("allowed_players").toArray()) {
        allowed << value.toString();
    }

    // Kept in order: the known ones, then anything allowed, then anything
    // open. That is also the order they win in when two play at once.
    QList<QPair<QString, QString>> names = kKnownPlayers;
    auto known = [&names](const QString &key) {
        for (const auto &pair : names) {
            if (pair.first == key) return true;
        }
        return false;
    };

    for (const QString &key : allowed) {
        if (!known(key)) names.append({key, key});
    }

    for (const QJsonValue &value : status.value("players").toArray()) {
        const QJsonObject player = value.toObject();
        const QString key = player.value("key").toString();
        if (!known(key)) names.append({key, player.value("name").toString()});
    }

    QList<Player> list;
    for (const auto &pair : names) {
        list.append({pair.first, pair.second, allowed.contains(pair.first)});
    }
    return list;
}

namespace {

Source source(const QJsonObject &status, const QString &name, const QString &key, const QString &problem) {
    QList<QJsonObject> users;
    for (const QJsonValue &value : activities(status)) {
        const QJsonObject activity = value.toObject();
        if (activity.value("source").toString() == key) users.append(activity);
    }

    if (!users.isEmpty()) return {name, "current", "Active", users};
    if (!problem.isEmpty()) return {name, "unknown", problem, users};
    return {name, "pending", "Idle", users};
}

} // namespace

QList<Source> sources(const QJsonObject &status) {
    QList<Source> list;

    const QString cider = status.value("cider").toString();
    list.append(source(status, "Cider", "cider", cider == "token-refused" ? "Token refused" : ""));

    const QString discord = status.value("discord").toString();
    list.append(source(status, "Discord rich presence", "discord", discord == "unavailable" ? "Unavailable" : ""));

    list.append(source(status, "Now Playing", "nowplaying", ""));

    return list;
}

} // namespace Texts
