#pragma once

// The tray icon and its menu: what is being shared, Pause, Open and Quit.
// Grey while nothing is being shared.

#include <QObject>

class MainWindow;
class QAction;
class QSystemTrayIcon;

class Tray : public QObject {
    Q_OBJECT

public:
    explicit Tray(MainWindow *window);

private:
    void render();

    MainWindow *m_window;
    QSystemTrayIcon *m_icon;
    QAction *m_now;
    QAction *m_pause;
};
