#include "MainWindow.h"
#include "Bridge.h"
#include "IconHelper.h"
#include "NowPlaying.h"
#include "Texts.h"

// Rust's functions, for start at login.
#include "aridan-presence-qt/src/ffi.cxxqt.h"

#include <QAction>
#include <QApplication>
#include <QCloseEvent>
#include <QDialog>
#include <QFrame>
#include <QHBoxLayout>
#include <QLabel>
#include <QMenuBar>
#include <QMessageBox>
#include <QPushButton>
#include <QScrollArea>
#include <QStatusBar>
#include <QTimer>
#include <QVBoxLayout>

namespace {

QString fromRust(const rust::String &text) {
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

} // namespace

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
    setWindowTitle("aridan-presence");

    // The pane fills the window between the menus and the status bar, in
    // the theme's field colour - white on Aero and Breeze's light look. It
    // is a scroll area, the same kind of widget as Device Manager's tree, so
    // the theme gives it the same sunken border.
    m_nowPlaying = new NowPlaying;
    m_nowPlaying->setContentsMargins(12, 12, 12, 12);
    connect(m_nowPlaying, &NowPlaying::settingsRequested, this, [this] { openSettings(SettingsDialog::Connections); });

    auto *pane = new QScrollArea;
    pane->setWidget(m_nowPlaying);
    pane->setWidgetResizable(true);
    pane->setBackgroundRole(QPalette::Base);
    pane->viewport()->setBackgroundRole(QPalette::Base);
    m_nowPlaying->setBackgroundRole(QPalette::Base);
    m_nowPlaying->setAutoFillBackground(true);

    setCentralWidget(pane);

    buildMenus();
    buildStatusBar();

    // Tall and narrow, a small player panel: the cover on top and what is
    // playing under it. A fixed size, like the other versions.
    setFixedSize(340, 520);

    connect(Presence::instance(), &Presence::statusChanged, this, &MainWindow::render);

    render();
}

// Plain text menus, as Windows 7's own tools had.
void MainWindow::buildMenus() {
    auto *fileMenu = menuBar()->addMenu("&File");
    fileMenu->addAction("&Close", QKeySequence::Close, this, &QWidget::close);
    fileMenu->addSeparator();
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
    optionsMenu->addAction("&Settings...", QKeySequence("Ctrl+,"), this, [this] { openSettings(); });

    auto *helpMenu = menuBar()->addMenu("&Help");
    helpMenu->addAction("&About aridan-presence", this, &MainWindow::showAbout);
}

// Whether this computer is on the site. How each source is doing is in
// Settings, on the Developer tab.
void MainWindow::buildStatusBar() {
    m_statusText = new QLabel;
    m_statusText->setTextFormat(Qt::PlainText);

    statusBar()->addWidget(m_statusText, 1);
}

void MainWindow::render() {
    const QJsonObject status = Presence::instance()->status();
    const bool setUp = status.value("set_up").toBool();
    const bool paused = status.value("paused").toBool();

    // The music takes the middle; or with none, whatever else there is.
    const QJsonObject music = Texts::musicOf(status);
    const QJsonObject other = Texts::otherOf(status);

    m_nowPlaying->setSetUp(setUp);
    if (music.isEmpty()) {
        m_nowPlaying->setActivities(other, QJsonObject());
    } else {
        m_nowPlaying->setActivities(music, other);
    }

    const QString sharing = Texts::sharing(status);
    m_statusText->setText(sharing);
    m_statusText->setToolTip(sharing);

    m_actPause->setChecked(paused);
    m_actPause->setEnabled(setUp);
}

// The same About as linux-devmgmt's: the icon beside the name, who made it
// and the version, a line, what it does, and OK.
void MainWindow::showAbout() {
    QDialog dialog(this);
    dialog.setWindowTitle("About aridan-presence");
    dialog.setFixedWidth(340);

    auto *icon = new QLabel;
    icon->setFixedSize(64, 64);
    icon->setPixmap(appIcon().pixmap(QSize(64, 64), devicePixelRatioF()));
    icon->setAlignment(Qt::AlignCenter);

    auto *info = new QVBoxLayout;
    info->addWidget(new QLabel("aridan-presence"));
    info->addWidget(new QLabel("@actuallyaridan"));
    info->addWidget(new QLabel("Version: " + QApplication::applicationVersion()));
    info->addStretch();

    auto *top = new QHBoxLayout;
    top->addWidget(icon);
    top->addSpacing(8);
    top->addLayout(info);
    top->addStretch();

    auto *line = new QFrame;
    line->setFrameShape(QFrame::HLine);
    line->setFrameShadow(QFrame::Sunken);

    auto paragraph = [](const QString &text) {
        auto *label = new QLabel(text);
        label->setWordWrap(true);
        label->setAlignment(Qt::AlignTop | Qt::AlignLeft);
        label->setContentsMargins(4, 4, 4, 4);
        return label;
    };

    auto *description = paragraph("aridan-presence shares what you're listening to and playing with "
                                  "aridan.net, so its music widget works without Discord running.");
    auto *credits = paragraph("Made with Qt 6 and Rust. The same app as on Windows and macOS, drawn "
                              "here by your desktop's own theme. Best enjoyed with AeroThemePlasma.");

    auto *ok = new QPushButton("OK");
    ok->setFixedWidth(80);
    ok->setDefault(true);
    connect(ok, &QPushButton::clicked, &dialog, &QDialog::accept);

    auto *buttons = new QHBoxLayout;
    buttons->addStretch();
    buttons->addWidget(ok);

    auto *layout = new QVBoxLayout(&dialog);
    layout->setContentsMargins(12, 12, 12, 12);
    layout->setSpacing(8);
    layout->addLayout(top);
    layout->addWidget(line);
    layout->addWidget(description);
    layout->addWidget(credits);
    layout->addSpacing(4);
    layout->addLayout(buttons);
    layout->setSizeConstraint(QLayout::SetFixedSize);

    dialog.exec();
}

void MainWindow::showAndRaise() {
    show();
    raise();
    activateWindow();

    // Nothing works until there is a token, and that is set in Settings,
    // so a first start opens it on Connections.
    if (!Presence::instance()->status().value("set_up").toBool()) {
        QTimer::singleShot(0, this, [this] { openSettings(SettingsDialog::Connections); });
    }
}

void MainWindow::openSettings(SettingsDialog::Tab tab) {
    if (m_settings) {
        m_settings->showTab(tab);
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

// Closing only hides the window. Exit, or Exit in the tray, ends the app.
void MainWindow::closeEvent(QCloseEvent *event) {
    if (m_settings) m_settings->close();
    hide();
    event->ignore();
}
