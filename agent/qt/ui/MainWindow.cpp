#include "MainWindow.h"
#include "Bridge.h"
#include "NowPlaying.h"
#include "SettingsDialog.h"
#include "Texts.h"

// Rust's functions, for start at login.
#include "aridan-presence-qt/src/ffi.cxxqt.h"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
#include <QMenuBar>
#include <QMessageBox>
#include <QLabel>
#include <QStatusBar>
#include <QTimer>

namespace {

QString fromRust(const rust::String &text) {
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

QLabel *plainLabel() {
    auto *label = new QLabel;
    label->setTextFormat(Qt::PlainText);
    return label;
}

} // namespace

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setWindowTitle("aridan-presence");

    // Windows Media Player's Now Playing, in the middle.
    m_nowPlaying = new NowPlaying;
    setCentralWidget(m_nowPlaying);

    buildMenus();
    buildStatusBar();

    // A fixed size, like the other versions.
    setFixedSize(680, 500);

    connect(Presence::instance(), &Presence::statusChanged, this, &MainWindow::render);

    render();
}

// Plain text menus, as Windows 7's own tools had.
void MainWindow::buildMenus() {
    auto *fileMenu = menuBar()->addMenu("&File");
    fileMenu->addAction("&Close", QKeySequence::Close, this, &QWidget::close);
    fileMenu->addAction("E&xit", QKeySequence::Quit, this, [] { Presence::instance()->quit(); });

    auto *optionsMenu = menuBar()->addMenu("&Options");

    m_actPause = optionsMenu->addAction("&Pause Sharing");
    m_actPause->setCheckable(true);
    m_actPause->setShortcut(QKeySequence("Ctrl+P"));
    connect(m_actPause, &QAction::triggered, this, [](bool checked) {
        Presence::instance()->setPaused(checked);
    });

    m_actAutostart = optionsMenu->addAction("Start When I &Log In");
    m_actAutostart->setCheckable(true);
    m_actAutostart->setChecked(autostart_enabled());
    connect(m_actAutostart, &QAction::triggered, this, [this](bool checked) {
        const QString problem = fromRust(set_autostart(checked));
        if (!problem.isEmpty()) {
            m_actAutostart->setChecked(!checked);
            QMessageBox::warning(this, "aridan-presence", problem);
        }
    });

    optionsMenu->addSeparator();
    optionsMenu->addAction("&Settings...", QKeySequence("Ctrl+,"), this, [this] { openSettings(0); });

    auto *helpMenu = menuBar()->addMenu("&Help");
    helpMenu->addAction("&About aridan-presence", this, [this] {
        QMessageBox::about(this, "About aridan-presence",
                           "<b>aridan-presence</b> " + QApplication::applicationVersion() +
                           "<p>Shares what I am listening to and playing with aridan.net, "
                           "so its music widget works without Discord.</p>");
    });
}

// "Nothing is playing" when there is nothing, on the left, and whether
// this computer is on the site, on the right. How each source is doing is
// in Settings, on the Developer tab.
void MainWindow::buildStatusBar() {
    m_playingText = plainLabel();
    m_statusText = plainLabel();

    statusBar()->addWidget(m_playingText, 1);
    statusBar()->addPermanentWidget(m_statusText);
}

void MainWindow::render() {
    const QJsonObject status = Presence::instance()->status();

    // The music takes the middle; or with none, whatever else there is.
    const QJsonObject music = Texts::musicOf(status);
    const QJsonObject other = Texts::otherOf(status);

    if (music.isEmpty()) {
        m_nowPlaying->setActivities(other, QJsonObject());
    } else {
        m_nowPlaying->setActivities(music, other);
    }

    if (music.isEmpty() && other.isEmpty()) {
        m_playingText->setText("Nothing is playing");
    } else {
        m_playingText->clear();
    }

    m_statusText->setText(Texts::sharing(status).text);

    m_actPause->setChecked(status.value("paused").toBool());
}

void MainWindow::showAndRaise() {
    show();
    raise();
    activateWindow();

    // Nothing works until there is a token, and that is set in Settings,
    // so a first start opens it on Connections.
    if (!Presence::instance()->status().value("set_up").toBool()) {
        QTimer::singleShot(0, this, [this] { openSettings(1); });
    }
}

void MainWindow::openSettings(int tab) {
    if (m_settings) {
        m_settings->raise();
        m_settings->activateWindow();
        return;
    }

    m_settings = new SettingsDialog(tab, this);
    m_settings->setAttribute(Qt::WA_DeleteOnClose);

    // Settings can change start at login too.
    connect(m_settings, &QDialog::finished, this, [this] {
        m_actAutostart->setChecked(autostart_enabled());
    });

    m_settings->show();
}

// Closing only hides the window. Exit, or Quit in the tray, ends the app.
void MainWindow::closeEvent(QCloseEvent *event) {
    if (m_settings) m_settings->close();
    hide();
    event->ignore();
}
