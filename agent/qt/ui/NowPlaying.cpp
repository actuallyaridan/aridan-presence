#include "NowPlaying.h"
#include "IconHelper.h"
#include "Texts.h"

#include <QApplication>
#include <QCommandLinkButton>
#include <QEvent>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QPainter>
#include <QProgressBar>
#include <QStackedWidget>
#include <QTimer>
#include <QVBoxLayout>

namespace {

// The cover's square, and the icon on it while there is no cover.
constexpr int kPicture = 220;
constexpr int kPictureIcon = 128;

// How fine the progress bar's steps are.
constexpr int kProgressSteps = 1000;

// Words never come in as markup: a song called "<b>" says "<b>".
QLabel *plainLabel(const QString &text = QString()) {
    auto *label = new QLabel(text);
    label->setTextFormat(Qt::PlainText);
    return label;
}

// Greyed the way the theme greys a hint in an empty field, which is softer
// than disabled text and follows light and dark.
QLabel *dimLabel(const QString &text = QString()) {
    auto *label = plainLabel(text);
    label->setForegroundRole(QPalette::PlaceholderText);
    return label;
}

// The window's own font, larger and/or bold - Windows 7's "main
// instruction" and the lines under it.
QFont scaled(const QFont &base, qreal factor, bool bold = false) {
    QFont font = base;
    font.setPointSizeF(base.pointSizeF() * factor);
    font.setBold(bold);
    return font;
}

QPixmap iconPixmap(const QIcon &icon, int size, const QWidget *widget) {
    return icon.pixmap(QSize(size, size), widget->devicePixelRatioF());
}

void setText(QLabel *label, const QString &text) {
    label->setText(text);
    label->setVisible(!text.isEmpty());
}

} // namespace

NowPlaying::NowPlaying(QWidget *parent) : QWidget(parent) {
    m_network = new QNetworkAccessManager(this);

    m_pages = new QStackedWidget;
    m_setUpPage = buildSetUpPage();
    m_nothingPage = buildNothingPage();
    m_activityPage = buildActivityPage();
    m_pages->addWidget(m_setUpPage);
    m_pages->addWidget(m_nothingPage);
    m_pages->addWidget(m_activityPage);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->addWidget(m_pages);

    auto *ticker = new QTimer(this);
    connect(ticker, &QTimer::timeout, this, &NowPlaying::tick);
    ticker->start(1000);

    applyLook();
    setActivities(QJsonObject(), QJsonObject());
}

QWidget *NowPlaying::buildSetUpPage() {
    auto *page = new QWidget;

    auto *icon = new QLabel;
    icon->setPixmap(iconPixmap(appIcon(), 64, this));

    auto *title = plainLabel("Set up aridan-presence");
    title->setObjectName("mainInstruction");

    auto *text = plainLabel("Add the server token in Settings to start sharing what you're "
                            "listening to and playing with aridan.net.");
    text->setWordWrap(true);

    // With the style's own command link arrow.
    auto *link = new QCommandLinkButton("Open Settings", "Connections > Server > Token");
    connect(link, &QCommandLinkButton::clicked, this, &NowPlaying::settingsRequested);

    for (QLabel *label : {title, text}) label->setAlignment(Qt::AlignHCenter);

    auto *layout = new QVBoxLayout(page);
    layout->setSpacing(8);
    layout->addStretch();
    layout->addWidget(icon, 0, Qt::AlignHCenter);
    layout->addSpacing(4);
    layout->addWidget(title);
    layout->addWidget(text);
    layout->addSpacing(8);
    layout->addWidget(link);
    layout->addStretch(2);
    return page;
}

QWidget *NowPlaying::buildNothingPage() {
    auto *page = new QWidget;

    auto *icon = new QLabel;
    const QIcon note = themeIcon({"audio-x-generic", "multimedia-audio-player", "media-optical-audio"});
    icon->setPixmap(note.pixmap(QSize(48, 48), devicePixelRatioF(), QIcon::Disabled));
    icon->setAlignment(Qt::AlignCenter);

    auto *text = dimLabel("Nothing is playing");
    text->setObjectName("nothing");
    text->setAlignment(Qt::AlignCenter);

    auto *layout = new QVBoxLayout(page);
    layout->addStretch();
    layout->addWidget(icon);
    layout->addSpacing(4);
    layout->addWidget(text);
    layout->addStretch();
    return page;
}

QWidget *NowPlaying::buildActivityPage() {
    auto *page = new QWidget;

    m_picture = new QLabel;
    m_picture->setFixedSize(kPicture, kPicture);
    m_picture->setAlignment(Qt::AlignCenter);

    // Which app it is playing in, under the album: "Spotify"
    m_heading = dimLabel();

    // The song, who it is by, and the album - or a game, what is going on
    // in it, and where.
    m_title = plainLabel();
    m_title->setObjectName("title");
    m_title->setWordWrap(true);
    m_title->setTextInteractionFlags(Qt::TextSelectableByMouse);

    m_subtitle = plainLabel();
    m_subtitle->setObjectName("subtitle");
    m_subtitle->setWordWrap(true);
    m_subtitle->setTextInteractionFlags(Qt::TextSelectableByMouse);

    m_third = dimLabel();
    m_third->setWordWrap(true);
    m_third->setTextInteractionFlags(Qt::TextSelectableByMouse);

    // 01:12 [=====------] -02:31
    m_elapsed = plainLabel();
    m_progress = new QProgressBar;
    m_progress->setRange(0, kProgressSteps);
    m_progress->setTextVisible(false);

    // Slimmer than a file copy's bar, which is what the theme's full height
    // looks like.
    m_progress->setFixedHeight(12);
    m_left = plainLabel();

    auto *progressRow = new QHBoxLayout;
    progressRow->setSpacing(8);
    progressRow->addWidget(m_elapsed);
    progressRow->addWidget(m_progress, 1);
    progressRow->addWidget(m_left);

    // Centred under the cover, wrapping onto a second line rather than
    // being cut off.
    for (QLabel *label : {m_title, m_subtitle, m_third, m_heading}) {
        label->setAlignment(Qt::AlignHCenter);
        label->setWordWrap(true);
    }

    auto *words = new QVBoxLayout;
    words->setSpacing(2);
    words->addWidget(m_title);
    words->addWidget(m_subtitle);
    words->addWidget(m_third);
    words->addWidget(m_heading);

    // The cover, the words and the bar, one under the other.
    auto *top = new QVBoxLayout;
    top->setSpacing(14);
    top->addWidget(m_picture, 0, Qt::AlignHCenter);
    top->addLayout(words);
    top->addLayout(progressRow);

    // The line underneath: a game alongside the music, say.
    m_otherRow = new QWidget;

    auto *line = new QFrame;
    line->setFrameShape(QFrame::HLine);
    line->setFrameShadow(QFrame::Sunken);

    m_otherIcon = new QLabel;
    m_otherName = plainLabel();
    m_otherName->setObjectName("otherName");
    m_otherLine = dimLabel();

    auto *otherWords = new QVBoxLayout;
    otherWords->setSpacing(0);
    otherWords->addWidget(m_otherName);
    otherWords->addWidget(m_otherLine);

    auto *otherInner = new QHBoxLayout;
    otherInner->setSpacing(10);
    otherInner->addWidget(m_otherIcon);
    otherInner->addLayout(otherWords, 1);

    auto *otherLayout = new QVBoxLayout(m_otherRow);
    otherLayout->setContentsMargins(0, 0, 0, 0);
    otherLayout->setSpacing(10);
    otherLayout->addWidget(line);
    otherLayout->addLayout(otherInner);

    // In the middle of the pane, with the line underneath at its bottom.
    auto *layout = new QVBoxLayout(page);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(12);
    layout->addStretch();
    layout->addLayout(top);
    layout->addStretch();
    layout->addWidget(m_otherRow);
    return page;
}

void NowPlaying::setSetUp(bool setUp) {
    if (setUp == m_setUp) return;
    m_setUp = setUp;
    setActivities(m_main, m_other);
}

void NowPlaying::setActivities(const QJsonObject &main, const QJsonObject &other) {
    m_main = main;
    m_other = other;

    if (!m_setUp) {
        m_pages->setCurrentWidget(m_setUpPage);
    } else if (main.isEmpty()) {
        m_pages->setCurrentWidget(m_nothingPage);
    } else {
        m_pages->setCurrentWidget(m_activityPage);
    }

    const QString picture = main.value("assets").toObject().value("large_image").toString();
    loadPicture(Texts::coverUrl(picture, 600));

    renderMain();
    renderOther();
    renderPicture();
    tick();
}

void NowPlaying::renderMain() {
    if (m_main.isEmpty()) return;

    const QString name = m_main.value("name").toString();
    const bool music = m_main.value("type").toInt() == Texts::Listening;

    // A game's name is already its title.
    setText(m_heading, music ? name : QString());

    // A song's name is its title; a game's own name is, with what is going
    // on in it under that.
    QString title;
    QString subtitle;
    QString third;
    if (music) {
        title = m_main.value("details").toString();
        subtitle = m_main.value("state").toString();
        third = m_main.value("assets").toObject().value("large_text").toString();
    } else {
        title = name;
        subtitle = m_main.value("details").toString();
        third = m_main.value("state").toString();
    }
    if (title.isEmpty()) title = name;

    // The album is often the song's name again, for a single.
    if (third == title) third.clear();

    setText(m_title, title);
    setText(m_subtitle, subtitle);
    setText(m_third, third);
}

void NowPlaying::renderOther() {
    m_otherRow->setVisible(!m_other.isEmpty());
    if (m_other.isEmpty()) return;

    const QString name = m_other.value("name").toString();
    const bool music = m_other.value("type").toInt() == Texts::Listening;

    m_otherIcon->setPixmap(iconPixmap(Texts::iconFor(name, music), 32, this));
    m_otherName->setText(Texts::heading(m_other.value("type").toInt()) + " " + name);
}

// The cover with a thin frame round it, or until there is one - or for an
// app that has none - the app's icon on a plain tile.
void NowPlaying::renderPicture() {
    const qreal ratio = devicePixelRatioF();
    QPixmap canvas(QSize(kPicture, kPicture) * ratio);
    canvas.setDevicePixelRatio(ratio);
    canvas.fill(Qt::transparent);

    QPainter painter(&canvas);
    painter.setRenderHint(QPainter::SmoothPixmapTransform);

    QRectF frame;
    if (!m_cover.isNull()) {
        const QSize fitted = m_cover.size().scaled(kPicture, kPicture, Qt::KeepAspectRatio);
        frame = QRectF((kPicture - fitted.width()) / 2.0, (kPicture - fitted.height()) / 2.0, fitted.width(),
                       fitted.height());
        painter.drawPixmap(frame, m_cover, m_cover.rect());
    } else {
        frame = QRectF(0, 0, kPicture, kPicture);
        painter.fillRect(frame, palette().color(QPalette::AlternateBase));

        const QString name = m_main.value("name").toString();
        const bool music = m_main.value("type").toInt() == Texts::Listening;
        const QRect middle((kPicture - kPictureIcon) / 2, (kPicture - kPictureIcon) / 2, kPictureIcon, kPictureIcon);
        Texts::iconFor(name, music).paint(&painter, middle);
    }

    painter.setPen(palette().color(QPalette::Mid));
    painter.setBrush(Qt::NoBrush);
    painter.drawRect(frame.adjusted(0.5, 0.5, -0.5, -0.5));
    painter.end();

    m_picture->setPixmap(canvas);
}

void NowPlaying::loadPicture(const QString &url) {
    if (url == m_coverUrl) return;
    m_coverUrl = url;
    m_cover = QPixmap();

    if (url.isEmpty()) return;

    QNetworkReply *reply = m_network->get(QNetworkRequest(QUrl(url)));

    connect(reply, &QNetworkReply::finished, this, [this, reply, url] {
        reply->deleteLater();

        // Another picture was asked for in the meantime.
        if (url != m_coverUrl) return;

        QPixmap pixmap;
        if (reply->error() == QNetworkReply::NoError && pixmap.loadFromData(reply->readAll())) {
            m_cover = pixmap;
            renderPicture();
        }
    });
}

void NowPlaying::tick() {
    // The main activity: a bar while there is a start and an end, and only
    // a clock without.
    if (!m_main.isEmpty()) {
        const Texts::Times times = Texts::timesOf(m_main);

        const bool bar = times.length > 0;
        m_progress->setVisible(bar);
        m_elapsed->setVisible(times.elapsed >= 0);
        m_left->setVisible(bar || times.remaining >= 0);

        if (bar) {
            m_progress->setValue(int(times.elapsed * kProgressSteps / times.length));
            m_elapsed->setText(Texts::clock(times.elapsed));
            m_left->setText("-" + Texts::clock(times.length - times.elapsed));
        } else {
            m_elapsed->setText(Texts::clock(times.elapsed) + " elapsed");
            m_left->setText(Texts::clock(times.remaining) + " left");
        }
    }

    // The line underneath: what is going on in it, and its clock.
    if (!m_other.isEmpty()) {
        QStringList parts;

        const QString details = m_other.value("details").toString();
        if (!details.isEmpty()) parts << details;

        const QString state = m_other.value("state").toString();
        if (!state.isEmpty()) parts << state;

        const Texts::Times times = Texts::timesOf(m_other);
        if (times.remaining >= 0) {
            parts << Texts::clock(times.remaining) + " left";
        } else if (times.elapsed >= 0) {
            parts << Texts::clock(times.elapsed) + " elapsed";
        }

        setText(m_otherLine, parts.join(QStringLiteral(" · ")));
    }
}

// Sizes follow the window's font, and the tile and frame its colours, so
// both are worked out again when the theme changes.
void NowPlaying::changeEvent(QEvent *event) {
    QWidget::changeEvent(event);

    switch (event->type()) {
    case QEvent::FontChange:
    case QEvent::PaletteChange:
    case QEvent::StyleChange:
        break;
    default:
        return;
    }

    applyLook();
}

void NowPlaying::applyLook() {
    for (QLabel *label : findChildren<QLabel *>("mainInstruction")) {
        label->setFont(scaled(font(), 1.5));
    }
    for (QLabel *label : findChildren<QLabel *>("title")) {
        label->setFont(scaled(font(), 1.35, true));
    }
    for (QLabel *label : findChildren<QLabel *>("subtitle")) {
        label->setFont(scaled(font(), 1.15));
    }
    for (QLabel *label : findChildren<QLabel *>("nothing")) {
        label->setFont(scaled(font(), 1.25));
    }
    m_otherName->setFont(scaled(font(), 1.0, true));

    renderPicture();
}
