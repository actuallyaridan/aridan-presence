#include "Tray.h"
#include "Bridge.h"
#include "MainWindow.h"
#include "Texts.h"

#include <QAction>
#include <QIcon>
#include <QMenu>
#include <QSystemTrayIcon>

Tray::Tray(MainWindow *window) : QObject(window), m_window(window) {
    auto *menu = new QMenu;

    m_now = menu->addAction("Nothing playing");
    m_now->setEnabled(false);

    menu->addSeparator();

    m_pause = menu->addAction("Pause sharing");
    m_pause->setCheckable(true);
    connect(m_pause, &QAction::triggered, this, [](bool checked) {
        Presence::instance()->setPaused(checked);
    });

    menu->addAction("Open aridan-presence", window, &MainWindow::showAndRaise);
    menu->addSeparator();
    menu->addAction("Quit", this, [] { Presence::instance()->quit(); });

    m_icon = new QSystemTrayIcon(this);
    m_icon->setContextMenu(menu);

    // A plain click opens the window; right-click shows the menu.
    connect(m_icon, &QSystemTrayIcon::activated, this, [this](QSystemTrayIcon::ActivationReason reason) {
        if (reason == QSystemTrayIcon::Trigger) m_window->showAndRaise();
    });

    connect(Presence::instance(), &Presence::statusChanged, this, &Tray::render);

    render();
    m_icon->show();
}

void Tray::render() {
    const QJsonObject status = Presence::instance()->status();
    const QString line = Texts::trayLine(status);

    m_now->setText(line);
    m_icon->setToolTip("aridan-presence\n" + line);
    m_pause->setChecked(status.value("paused").toBool());

    const bool sharing = status.value("set_up").toBool() && !status.value("paused").toBool();
    if (sharing) {
        m_icon->setIcon(QIcon(":/icons/tray.png"));
    } else {
        m_icon->setIcon(QIcon(":/icons/tray-paused.png"));
    }
}
