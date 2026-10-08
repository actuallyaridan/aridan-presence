// A classic include guard rather than #pragma once: cxx copies this file
// next to the code it makes, so it gets included under two paths, which
// #pragma once takes for two different files.
#ifndef ARIDAN_PRESENCE_BRIDGE_H
#define ARIDAN_PRESENCE_BRIDGE_H

// The C++ half of the bridge to Rust (src/ffi.rs).
//
// Rust calls the free functions at the bottom - from its own threads, so
// they only pass a message to Qt's thread, where Presence takes it from
// there. The window and the tray listen to Presence's signals, and ask Rust
// for anything else through the functions in ffi.rs.

#include "rust/cxx.h"

#include <QJsonObject>
#include <QObject>

class Presence : public QObject {
    Q_OBJECT

public:
    // There is one, made by run_application() once Qt has started.
    explicit Presence(QObject *parent = nullptr);
    static Presence *instance();

    // The engine's latest status - see core/src/state.rs for its fields.
    QJsonObject status() const { return m_status; }

    // The settings as saved, read fresh from Rust.
    QJsonObject config() const;

    // Saves settings. Gives back nothing if that worked, or why not.
    QString saveConfig(const QJsonObject &config);

    void setPaused(bool paused);

    // Takes this computer off the site, then quits.
    void quit();

    // Used by post_status() and friends; not for anything else.
    void receiveStatus(const QString &json);
    void receiveShow();
    void receiveQuit();

signals:
    void statusChanged();
    void showRequested();

private:
    QJsonObject m_status;
};

// Starts Qt, the tray and the window, and runs until Quit.
int run_application();

// Called by Rust, from any thread.
void post_status(rust::String json);
void post_show();
void post_quit();

#endif
