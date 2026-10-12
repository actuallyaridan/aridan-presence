#pragma once

// The middle of the window, made of Qt's own widgets so the desktop's theme
// draws it - Aero on a Windows 7 look, Breeze on a plain KDE, whatever else
// elsewhere. Only the thin frame round the cover is drawn here.
//
// It shows one of three pages:
//
//   Set up        first start, before there is a token: what to do, and a
//                 command link to Settings
//   Nothing       a line saying nothing is playing
//   Activity      the cover on top, under it what is playing, who by, and
//                 a progress bar between how far in it is and how long is
//                 left. Anything else going on - a game alongside the
//                 music - gets a line of its own at the bottom.

#include <QJsonObject>
#include <QPixmap>
#include <QWidget>

class QLabel;
class QNetworkAccessManager;
class QProgressBar;
class QStackedWidget;

class NowPlaying : public QWidget {
    Q_OBJECT

public:
    explicit NowPlaying(QWidget *parent = nullptr);

    // `main` takes the middle; `other` the line underneath. Either may be
    // empty, and with both empty it says nothing is playing.
    void setActivities(const QJsonObject &main, const QJsonObject &other);

    void setSetUp(bool setUp);

signals:
    void settingsRequested();

protected:
    void changeEvent(QEvent *event) override;

private:
    QWidget *buildSetUpPage();
    QWidget *buildNothingPage();
    QWidget *buildActivityPage();

    // The larger fonts, and the cover's tile and frame, from the theme.
    void applyLook();

    void loadPicture(const QString &url);
    void renderPicture();
    void renderMain();
    void renderOther();

    // The clocks and the bar move every second, between status updates.
    void tick();

    QStackedWidget *m_pages;
    QWidget *m_setUpPage;
    QWidget *m_nothingPage;
    QWidget *m_activityPage;

    bool m_setUp = true;
    QJsonObject m_main;
    QJsonObject m_other;

    // The main activity
    QLabel *m_picture;
    QLabel *m_heading;
    QLabel *m_title;
    QLabel *m_subtitle;
    QLabel *m_third;
    QLabel *m_elapsed;
    QProgressBar *m_progress;
    QLabel *m_left;

    // The line underneath
    QWidget *m_otherRow;
    QLabel *m_otherIcon;
    QLabel *m_otherName;
    QLabel *m_otherLine;

    QNetworkAccessManager *m_network;

    // The cover or game picture, once it has arrived, and the address it
    // came from - or is on its way from.
    QPixmap m_cover;
    QString m_coverUrl;
};
