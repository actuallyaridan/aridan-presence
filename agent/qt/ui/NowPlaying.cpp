#include "NowPlaying.h"
#include "Texts.h"

#include <QDateTime>
#include <QLinearGradient>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPainter>
#include <QTimer>

namespace {

constexpr int kSquare = 200;

// Where the bottom part starts: the progress line, then the capsule.
constexpr int kProgressFromBottom = 62;
constexpr int kCapsuleHeight = 40;
constexpr int kCapsuleFromBottom = 12;

const QColor kText(255, 255, 255);
const QColor kDimText(255, 255, 255, 165);

// How far in, and how long, in milliseconds. -1 where not known.
struct Times {
    qint64 elapsed = -1;
    qint64 length = -1;
    qint64 remaining = -1;
};

Times timesOf(const QJsonObject &activity) {
    const QJsonObject times = activity.value("timestamps").toObject();
    const qint64 start = times.value("start").toInteger();
    const qint64 end = times.value("end").toInteger();
    const qint64 now = QDateTime::currentMSecsSinceEpoch();

    Times out;
    if (start > 0 && end > start) {
        out.length = end - start;
        out.elapsed = qBound<qint64>(0, now - start, out.length);
    } else if (end > 0) {
        out.remaining = qMax<qint64>(0, end - now);
    } else if (start > 0) {
        out.elapsed = qMax<qint64>(0, now - start);
    }
    return out;
}

QFont sized(QFont font, qreal factor, bool bold = false) {
    font.setPointSizeF(font.pointSizeF() * factor);
    font.setBold(bold);
    return font;
}

} // namespace

NowPlaying::NowPlaying(QWidget *parent) : QWidget(parent) {
    m_network = new QNetworkAccessManager(this);

    // The clocks and the line move every second, between status updates.
    auto *ticker = new QTimer(this);
    connect(ticker, &QTimer::timeout, this, qOverload<>(&QWidget::update));
    ticker->start(1000);
}

void NowPlaying::setActivities(const QJsonObject &main, const QJsonObject &corner) {
    m_main = main;
    m_corner = corner;
    m_music = main.value("type").toInt() == Texts::Listening;

    const QJsonObject assets = main.value("assets").toObject();
    loadPicture(assets.value("large_image").toString());

    update();
}

void NowPlaying::loadPicture(const QString &url) {
    if (url == m_pictureUrl) return;
    m_pictureUrl = url;
    m_picture = QPixmap();

    if (url.isEmpty()) return;

    QNetworkReply *reply = m_network->get(QNetworkRequest(QUrl(url)));

    connect(reply, &QNetworkReply::finished, this, [this, reply, url] {
        reply->deleteLater();

        // Another picture was asked for in the meantime.
        if (url != m_pictureUrl) return;

        QPixmap pixmap;
        if (reply->error() == QNetworkReply::NoError && pixmap.loadFromData(reply->readAll())) {
            m_picture = pixmap;
            update();
        }
    });
}

void NowPlaying::paintEvent(QPaintEvent *) {
    QPainter painter(this);
    painter.setRenderHint(QPainter::Antialiasing);
    painter.setRenderHint(QPainter::SmoothPixmapTransform);

    // The backdrop, scaled to cover the view and cropped to fit, so it is
    // never stretched out of shape.
    static const QPixmap backdrop(":/images/background.jpg");
    const QSize covering = backdrop.size().scaled(size(), Qt::KeepAspectRatioByExpanding);
    painter.drawPixmap(QRect(QPoint((width() - covering.width()) / 2, (height() - covering.height()) / 2),
                             covering),
                       backdrop);

    // Nothing to show: just the backdrop. The status bar says so.
    if (m_main.isEmpty()) return;

    // The square sits in the middle of what is above the bottom part.
    const int above = height() - kProgressFromBottom;
    const QRect square((width() - kSquare) / 2, (above - kSquare) / 2, kSquare, kSquare);

    paintPicture(painter, square);
    paintProgress(painter);
    paintCapsule(painter);
    paintTopLeft(painter);
    paintCorner(painter);
}

// The cover, or while there is none, a pane of frosted glass with the
// app's icon on it, as Media Player shows its music note.
void NowPlaying::paintPicture(QPainter &painter, const QRect &square) {
    if (!m_picture.isNull()) {
        const QSize fitted = m_picture.size().scaled(square.size(), Qt::KeepAspectRatio);
        const QRect target(square.center().x() - fitted.width() / 2 + 1,
                           square.center().y() - fitted.height() / 2 + 1, fitted.width(), fitted.height());
        painter.drawPixmap(target, m_picture);

        painter.setPen(QColor(255, 255, 255, 70));
        painter.setBrush(Qt::NoBrush);
        painter.drawRect(QRectF(target).adjusted(0.5, 0.5, -0.5, -0.5));
        return;
    }

    QLinearGradient frost(square.topLeft(), square.bottomRight());
    frost.setColorAt(0.0, QColor(255, 255, 255, 60));
    frost.setColorAt(1.0, QColor(255, 255, 255, 18));

    painter.setPen(QColor(255, 255, 255, 90));
    painter.setBrush(frost);
    painter.drawRect(QRectF(square).adjusted(0.5, 0.5, -0.5, -0.5));

    const QIcon icon = Texts::iconFor(m_main.value("name").toString(), m_music);
    const int size = kSquare / 2;
    icon.paint(&painter, QRect(square.center().x() - size / 2 + 1, square.center().y() - size / 2 + 1, size, size));
}

// Media Player's seek line: a dark groove across the window with a blue,
// glowing fill for how far in the song is.
void NowPlaying::paintProgress(QPainter &painter) {
    const Times times = timesOf(m_main);
    if (times.length <= 0) return;

    const QRectF groove(10.5, height() - kProgressFromBottom + 0.5, width() - 21, 4);

    painter.setPen(QColor(255, 255, 255, 45));
    painter.setBrush(QColor(0, 0, 0, 150));
    painter.drawRoundedRect(groove, 2, 2);

    const qreal fraction = qreal(times.elapsed) / qreal(times.length);
    QRectF fill = groove.adjusted(1, 1, -1, -1);
    fill.setWidth(fill.width() * fraction);

    QLinearGradient blue(fill.topLeft(), fill.bottomLeft());
    blue.setColorAt(0.0, QColor("#8FD3FF"));
    blue.setColorAt(1.0, QColor("#1C7FD6"));

    painter.setPen(Qt::NoPen);
    painter.setBrush(blue);
    painter.drawRect(fill);
}

// The capsule Media Player keeps its buttons in, here see-through and
// holding the name and who it is by, with the clocks either side of it.
void NowPlaying::paintCapsule(QPainter &painter) {
    QString title = m_main.value("details").toString();
    QString subtitle = m_main.value("state").toString();

    // A game's own name is its title, with what is going on under it.
    if (!m_music) {
        subtitle = title.isEmpty() ? subtitle : title;
        title = m_main.value("name").toString();
    }
    if (title.isEmpty()) title = m_main.value("name").toString();

    const QFont titleFont = sized(font(), 1.0, true);
    const QFont subtitleFont = sized(font(), 0.9);
    const QFontMetrics titleMetrics(titleFont);
    const QFontMetrics subtitleMetrics(subtitleFont);

    const int widest = qMax(titleMetrics.horizontalAdvance(title), subtitleMetrics.horizontalAdvance(subtitle));
    const int capsuleWidth = qBound(240, widest + 56, width() - 170);

    const QRectF capsule((width() - capsuleWidth) / 2.0 + 0.5,
                         height() - kCapsuleFromBottom - kCapsuleHeight + 0.5,
                         capsuleWidth - 1, kCapsuleHeight - 1);
    const qreal radius = kCapsuleHeight / 2.0;

    // Plain and see-through, so the backdrop shows through it.
    painter.setPen(QColor(255, 255, 255, 40));
    painter.setBrush(QColor(120, 128, 138, 90));
    painter.drawRoundedRect(capsule, radius, radius);

    // The words, elided where the capsule is not wide enough.
    const QRectF inside = capsule.adjusted(22, 3, -22, -3);

    if (subtitle.isEmpty()) {
        painter.setFont(titleFont);
        painter.setPen(kText);
        painter.drawText(inside, Qt::AlignCenter, titleMetrics.elidedText(title, Qt::ElideRight, int(inside.width())));
    } else {
        const QRectF top(inside.left(), inside.top(), inside.width(), inside.height() / 2);
        const QRectF bottom(inside.left(), inside.center().y(), inside.width(), inside.height() / 2);

        painter.setFont(titleFont);
        painter.setPen(kText);
        painter.drawText(top, Qt::AlignHCenter | Qt::AlignBottom,
                         titleMetrics.elidedText(title, Qt::ElideRight, int(inside.width())));

        painter.setFont(subtitleFont);
        painter.setPen(kDimText);
        painter.drawText(bottom, Qt::AlignHCenter | Qt::AlignTop,
                         subtitleMetrics.elidedText(subtitle, Qt::ElideRight, int(inside.width())));
    }

    // How far in on the left, as Media Player has it; how long, or how long
    // is left, on the right.
    const Times times = timesOf(m_main);
    painter.setFont(font());
    painter.setPen(kText);

    const QRectF left(0, capsule.top(), capsule.left() - 14, capsule.height());
    const QRectF right(capsule.right() + 14, capsule.top(), width() - capsule.right() - 14, capsule.height());

    if (times.elapsed >= 0) {
        painter.drawText(left, Qt::AlignRight | Qt::AlignVCenter, Texts::clock(times.elapsed));
    }
    if (times.length > 0) {
        painter.drawText(right, Qt::AlignLeft | Qt::AlignVCenter, Texts::clock(times.length));
    } else if (times.remaining >= 0) {
        painter.drawText(right, Qt::AlignLeft | Qt::AlignVCenter, "-" + Texts::clock(times.remaining));
    }
}

// Which app the music is from, where Media Player writes the song's name.
void NowPlaying::paintTopLeft(QPainter &painter) {
    if (!m_music) return;

    const QString name = m_main.value("name").toString();
    const QIcon icon = Texts::iconFor(name, true);

    icon.paint(&painter, QRect(10, 9, 16, 16));

    painter.setFont(font());
    painter.setPen(kText);
    painter.drawText(QRect(32, 8, width() / 2, 18), Qt::AlignLeft | Qt::AlignVCenter, name);
}

// Anything else going on - a game alongside the music - small, in the top
// right corner: its icon, what it is, and its clock.
void NowPlaying::paintCorner(QPainter &painter) {
    if (m_corner.isEmpty()) return;

    const QString name = m_corner.value("name").toString();
    QString line = name;

    const QString details = m_corner.value("details").toString();
    if (!details.isEmpty()) line += " - " + details;

    const Times times = timesOf(m_corner);
    QString clock;
    if (times.remaining >= 0) {
        clock = Texts::clock(times.remaining) + " left";
    } else if (times.elapsed >= 0) {
        clock = Texts::clock(times.elapsed);
    }

    const QFontMetrics metrics(font());
    const int maxWidth = width() / 2 - 40;
    line = metrics.elidedText(line, Qt::ElideRight, maxWidth);

    painter.setFont(font());

    const int lineWidth = metrics.horizontalAdvance(line);
    const QRect lineRect(width() - 10 - lineWidth, 8, lineWidth, 18);
    painter.setPen(kText);
    painter.drawText(lineRect, Qt::AlignRight | Qt::AlignVCenter, line);

    Texts::iconFor(name, false).paint(&painter, QRect(lineRect.left() - 22, 9, 16, 16));

    if (!clock.isEmpty()) {
        painter.setPen(kDimText);
        painter.drawText(QRect(width() - 10 - maxWidth, 26, maxWidth, 16), Qt::AlignRight | Qt::AlignVCenter, clock);
    }
}
