#pragma once

// The window: a menu bar, Windows Media Player's Now Playing in the middle
// (NowPlaying.h), and a status bar saying whether this computer is on the
// site.
// Closing it only hides it; the app carries on in the tray.

#include <QMainWindow>
#include <QPointer>

class NowPlaying;
class QAction;
class QLabel;
class SettingsDialog;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);

    void showAndRaise();

    // Opens Settings on one of its tabs, or brings it forward if it is
    // already open.
    void openSettings(int tab = 0);

protected:
    void closeEvent(QCloseEvent *event) override;

private:
    void buildMenus();
    void buildStatusBar();
    void render();

    NowPlaying *m_nowPlaying;

    QLabel *m_playingText;
    QLabel *m_statusText;

    QAction *m_actPause;
    QAction *m_actAutostart;

    QPointer<SettingsDialog> m_settings;
};
