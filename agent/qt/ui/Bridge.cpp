#include "Bridge.h"
#include "IconHelper.h"
#include "MainWindow.h"
#include "Tray.h"

// Made by cxx from src/ffi.rs: Rust's functions, callable from here.
#include "aridan-presence-qt/src/ffi.cxxqt.h"

#include <QApplication>
#include <QJsonDocument>
#include <QPointer>
#include <atomic>

namespace {

// Set once Qt is running. Until then there is nothing to tell, and Rust's
// messages are dropped - the window reads the status itself when it opens.
std::atomic<Presence *> s_presence{nullptr};

QString fromRust(const rust::String &text) {
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

QJsonObject parseObject(const QString &json) {
    return QJsonDocument::fromJson(json.toUtf8()).object();
}

// Runs `action` on Qt's thread, with the one Presence, if there is one yet.
template <typename Action>
void onQtThread(Action action) {
    Presence *presence = s_presence.load();
    if (!presence) return;

    QMetaObject::invokeMethod(presence, [presence, action] { action(presence); },
                              Qt::QueuedConnection);
}

} // namespace

Presence::Presence(QObject *parent) : QObject(parent) {
    m_status = parseObject(fromRust(status_json()));
}

Presence *Presence::instance() {
    return s_presence.load();
}

QJsonObject Presence::config() const {
    return parseObject(fromRust(config_json()));
}

QString Presence::saveConfig(const QJsonObject &config) {
    const QByteArray json = QJsonDocument(config).toJson(QJsonDocument::Compact);
    return fromRust(save_config(rust::Str(json.constData(), json.size())));
}

void Presence::setPaused(bool paused) {
    set_paused(paused);
}

void Presence::quit() {
    request_quit();
}

void Presence::receiveStatus(const QString &json) {
    const QJsonObject status = parseObject(json);
    if (status == m_status) return;

    m_status = status;
    emit statusChanged();
}

void Presence::receiveShow() {
    emit showRequested();
}

void Presence::receiveQuit() {
    QApplication::quit();
}

int run_application() {
    // Qt keeps a reference to these for as long as it runs, so they have to
    // outlive this function's first line. `static` does that.
    static int argc = 1;
    static char name[] = "aridan-presence";
    static char *argv[] = { name, nullptr };

    QApplication app(argc, argv);
    app.setOrganizationName("actuallyaridan");
    app.setApplicationName("aridan-presence");
    app.setApplicationVersion(fromRust(app_version()));
    app.setDesktopFileName("aridan-presence");
    app.setWindowIcon(appIcon());

    // Closing the window leaves the app in the tray.
    app.setQuitOnLastWindowClosed(false);

    Presence presence;
    s_presence.store(&presence);

    MainWindow window;
    Tray tray(&window);

    QObject::connect(&presence, &Presence::showRequested, &window, &MainWindow::showAndRaise);

    // Started at login, it stays in the tray - unless it is not set up yet,
    // since Settings is where that gets done.
    const bool setUp = presence.status().value("set_up").toBool();
    if (!start_minimized() || !setUp) {
        window.showAndRaise();
    }

    const int code = app.exec();

    s_presence.store(nullptr);
    return code;
}

void post_status(rust::String json) {
    const QString text = fromRust(json);
    onQtThread([text](Presence *presence) { presence->receiveStatus(text); });
}

void post_show() {
    onQtThread([](Presence *presence) { presence->receiveShow(); });
}

void post_quit() {
    onQtThread([](Presence *presence) { presence->receiveQuit(); });
}
