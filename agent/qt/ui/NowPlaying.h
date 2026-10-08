#pragma once

// The middle of the window, after Windows Media Player 12's Now Playing
// view: its dark blue backdrop, the album art in a framed square in the
// middle, a thin blue progress line across the bottom, and under it -
// where Media Player has its controls - a dark capsule with the song's
// name and artist, between how far in it is and how long it is.
//
// One activity takes the middle: the music, or if there is none the game
// or app. Anything else - a game alongside the music - sits small in the
// top right corner. Media Player drew this view itself rather than out of
// standard controls, and so does this.

#include <QJsonObject>
#include <QPixmap>
#include <QWidget>

class QNetworkAccessManager;

class NowPlaying : public QWidget {
    Q_OBJECT

public:
    explicit NowPlaying(QWidget *parent = nullptr);

    // `main` takes the middle; `corner` the top right. Either may be empty.
    void setActivities(const QJsonObject &main, const QJsonObject &corner);

protected:
    void paintEvent(QPaintEvent *event) override;

private:
    void loadPicture(const QString &url);

    void paintPicture(QPainter &painter, const QRect &square);
    void paintProgress(QPainter &painter);
    void paintCapsule(QPainter &painter);
    void paintTopLeft(QPainter &painter);
    void paintCorner(QPainter &painter);

    QJsonObject m_main;
    QJsonObject m_corner;
    bool m_music = false;

    QNetworkAccessManager *m_network;

    // The cover or game picture, once it has arrived, and the address it
    // came from - or is on its way from.
    QPixmap m_picture;
    QString m_pictureUrl;
};
