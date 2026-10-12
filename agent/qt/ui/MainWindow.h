#pragma once

// The window, laid out like Windows 7's own tools: a menu bar - Pause
// Sharing and Settings are in Options - the Now Playing pane (NowPlaying.h)
// in a frame, and a status bar saying whether this computer is on the site.
// All of it is Qt's own widgets, so the desktop's theme draws it.
//
// Closing it only hides it; the app carries on in the tray.

#include "SettingsDialog.h"

#include <QMainWindow>
#include <QPointer>

class NowPlaying;
class QAction;
class QLabel;

class MainWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit MainWindow(QWidget *parent = nullptr);

    void showAndRaise();

    // Opens Settings on one of its tabs, or brings it forward if it is
    // already open.
    void openSettings(SettingsDialog::Tab tab = SettingsDialog::General);

protected:
    void closeEvent(QCloseEvent *event) override;

private:
    void buildMenus();
    void buildStatusBar();
    void render();
    void showAbout();

    NowPlaying *m_nowPlaying;

    QLabel *m_statusText;

    QAction *m_actPause;
    QAction *m_actAutostart;

    QPointer<SettingsDialog> m_settings;
};
