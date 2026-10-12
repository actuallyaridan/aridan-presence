#pragma once

// The tray icon and its menu: Open, what is being shared, Pause, Settings
// and Exit. Grey while nothing is being shared.

#include <QObject>

class MainWindow;
class QAction;
class QMenu;
class QSystemTrayIcon;

class Tray : public QObject {
    Q_OBJECT

public:
    explicit Tray(MainWindow *window);
    ~Tray() override;

private:
    void render();

    MainWindow *m_window;
    QSystemTrayIcon *m_icon;

    // A menu has no parent to delete it - a widget's parent would make it a
    // part of that widget - so the tray does.
    QMenu *m_menu;
    QAction *m_now;
    QAction *m_pause;
};
