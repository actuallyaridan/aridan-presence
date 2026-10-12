#include "Tray.h"
#include "Bridge.h"
#include "MainWindow.h"
#include "Texts.h"

#include <QAction>
#include <QIcon>
#include <QMenu>
#include <QSystemTrayIcon>

// The menu, as Windows 7's tray menus have them - the default action, which
// a click on the icon does too, in bold at the top:
//
//   Open aridan-presence
//   ─────────────
//   Popila - Severina          what is being shared (not clickable)
//   ─────────────
//   ✓ Pause Sharing
//   Settings...
//   ─────────────
//   Exit
Tray::Tray(MainWindow *window) : QObject(window), m_window(window) {
    m_menu = new QMenu;

    QAction *open = m_menu->addAction("&Open aridan-presence", window, &MainWindow::showAndRaise);
    m_menu->setDefaultAction(open);

    m_menu->addSeparator();

    m_now = m_menu->addAction("Nothing playing");
    m_now->setEnabled(false);

    m_menu->addSeparator();

    m_pause = m_menu->addAction("&Pause Sharing");
    m_pause->setCheckable(true);
    connect(m_pause, &QAction::triggered, this, [](bool checked) {
        Presence::instance()->setPaused(checked);
    });

    m_menu->addAction("&Settings...", window, [window] { window->openSettings(); });
    m_menu->addSeparator();
    m_menu->addAction("E&xit", this, [] { Presence::instance()->quit(); });

    m_icon = new QSystemTrayIcon(this);
    m_icon->setContextMenu(m_menu);

    // A plain click opens the window; right-click shows the menu.
    connect(m_icon, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger) m_window->showAndRaise();
    });

    connect(Presence::instance(), &Presence::statusChanged, this, &Tray::render);

    render();
    m_icon->show();
}

Tray::~Tray() {
    delete m_menu;
}

void Tray::render() {
    const QJsonObject status = Presence::instance()->status();
    const QString line = Texts::trayLine(status);
    const bool setUp = status.value("set_up").toBool();
    const bool paused = status.value("paused").toBool();

    m_now->setText(line);
    m_icon->setToolTip("aridan-presence\n" + line);
    m_pause->setChecked(paused);
    m_pause->setEnabled(setUp);

    // Grey while nothing is being shared.
    if (setUp && !paused) {
        m_icon->setIcon(QIcon(":/icons/tray.png"));
    } else {
        m_icon->setIcon(QIcon(":/icons/tray-paused.png"));
    }
}
