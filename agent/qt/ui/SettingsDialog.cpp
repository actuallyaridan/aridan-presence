#include "SettingsDialog.h"
#include "Bridge.h"
#include "IconHelper.h"
#include "Texts.h"

// Rust's functions: the log, start at login, where the settings live.
#include "aridan-presence-qt/src/ffi.cxxqt.h"

#include <QCheckBox>
#include <QDateTime>
#include <QDesktopServices>
#include <QDialogButtonBox>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QHeaderView>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QListWidget>
#include <QPainter>
#include <QPushButton>
#include <QShowEvent>
#include <QTabWidget>
#include <QTreeWidget>
#include <QUrl>
#include <QVBoxLayout>

namespace {

// linux-devmgmt's Properties dialog: a fixed width, and at least this tall.
constexpr int kWidth = 420;
constexpr int kMinimumHeight = 470;

QString fromRust(const rust::String &text) {
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

QLabel *plainLabel(const QString &text = QString()) {
    auto *label = new QLabel(text);
    label->setTextFormat(Qt::PlainText);
    return label;
}

// A line of explanation in a group, wrapped to its width.
QLabel *hintLabel(const QString &text) {
    auto *label = plainLabel(text);
    label->setWordWrap(true);
    return label;
}

// A tab's own layout, with linux-devmgmt's margins.
QVBoxLayout *tabLayout(QWidget *tab) {
    auto *layout = new QVBoxLayout(tab);
    layout->setContentsMargins(8, 8, 8, 8);
    layout->setSpacing(12);
    return layout;
}

// A list in the thin frame linux-devmgmt puts its text boxes in: a one
// pixel line in the theme's midlight colour, round its base colour.
QFrame *framed(QAbstractScrollArea *view) {
    auto *frame = new QFrame;
    frame->setObjectName("listFrame");
    frame->setStyleSheet(
        "QFrame#listFrame { border: 1px solid palette(midlight); background-color: palette(base); }");

    view->setFrameShape(QFrame::NoFrame);

    auto *layout = new QVBoxLayout(frame);
    layout->setContentsMargins(1, 1, 1, 1);
    layout->setSpacing(0);
    layout->addWidget(view);
    return frame;
}

} // namespace

SettingsDialog::SettingsDialog(Tab tab, QWidget *parent) : QDialog(parent) {
    setWindowTitle("aridan-presence Settings");
    setWindowFlags(Qt::Dialog | Qt::CustomizeWindowHint | Qt::WindowTitleHint | Qt::WindowCloseButtonHint);
    setFixedWidth(kWidth);
    resize(kWidth, kMinimumHeight);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    m_tabs = new QTabWidget;
    m_tabs->addTab(buildGeneralTab(), "General");
    m_tabs->addTab(buildConnectionsTab(), "Connections");
    m_tabs->addTab(buildDeveloperTab(), "Developer");
    m_tabs->addTab(buildLogTab(), "Log");
    showTab(tab);
    layout->addWidget(m_tabs, 1);

    // A problem with the settings file at startup, or with saving.
    m_problem = hintLabel(QString());
    layout->addWidget(m_problem);
    showProblem(Presence::instance()->status().value("config_error").toString());

    m_buttons = new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel | QDialogButtonBox::Apply);
    m_buttons->button(QDialogButtonBox::Apply)->setEnabled(false);
    layout->addWidget(m_buttons);

    connect(m_buttons, &QDialogButtonBox::accepted, this, [this] {
        if (apply()) accept();
    });
    connect(m_buttons, &QDialogButtonBox::rejected, this, &QDialog::reject);
    connect(m_buttons->button(QDialogButtonBox::Apply), &QPushButton::clicked, this, [this] { apply(); });

    // Sources and the log follow along while the dialog is open.
    connect(Presence::instance(), &Presence::statusChanged, this, [this] {
        renderSources();
        renderLog();
    });

    renderSources();
    renderLog();
}

void SettingsDialog::showTab(Tab tab) {
    m_tabs->setCurrentIndex(tab);
}

// As tall as the tallest tab needs, and never shorter than linux-devmgmt's,
// then fixed there. Wrapped text can claim less height than it takes, so
// the floor keeps it from being cut off.
void SettingsDialog::showEvent(QShowEvent *event) {
    QDialog::showEvent(event);

    if (!m_sized) {
        m_sized = true;
        adjustSize();
        setFixedSize(width(), qMax(height(), kMinimumHeight));
    }
}

QWidget *SettingsDialog::buildGeneralTab() {
    auto *page = new QWidget;
    auto *layout = tabLayout(page);

    m_autostart = new QCheckBox("&Start when I log in");
    m_autostart->setChecked(autostart_enabled());
    connect(m_autostart, &QCheckBox::toggled, this, &SettingsDialog::markChanged);
    layout->addWidget(m_autostart);

    auto *apps = new QGroupBox("Apps that may be shared");
    auto *appsLayout = new QVBoxLayout(apps);

    // A box to tick by each app, as Windows 7's "Turn Windows features on or
    // off" has.
    m_players = new QListWidget;
    m_players->setIconSize(QSize(16, 16));
    m_players->setUniformItemSizes(true);

    const QJsonObject config = Presence::instance()->config();
    const QJsonObject status = Presence::instance()->status();

    for (const Texts::Player &player : Texts::playerList(config, status)) {
        if (!player.shown) {
            if (player.allowed) m_hiddenAllowed << player.key;
            continue;
        }

        auto *item = new QListWidgetItem(Texts::iconFor(player.name, true), player.name, m_players);
        item->setFlags(Qt::ItemIsEnabled | Qt::ItemIsSelectable | Qt::ItemIsUserCheckable);
        item->setCheckState(player.allowed ? Qt::Checked : Qt::Unchecked);
        item->setData(Qt::UserRole, player.key);
    }

    connect(m_players, &QListWidget::itemChanged, this, &SettingsDialog::markChanged);

    appsLayout->addWidget(framed(m_players), 1);

    layout->addWidget(apps, 1);
    return page;
}

QWidget *SettingsDialog::buildConnectionsTab() {
    auto *page = new QWidget;
    auto *layout = tabLayout(page);

    const QJsonObject config = Presence::instance()->config();

    auto field = [this](const QString &text) {
        auto *edit = new QLineEdit(text);
        connect(edit, &QLineEdit::textEdited, this, &SettingsDialog::markChanged);
        return edit;
    };

    // Server
    auto *server = new QGroupBox("Server");
    auto *serverForm = new QFormLayout(server);

    m_token = field(config.value("token").toString());
    m_token->setEchoMode(QLineEdit::Password);
    m_server = field(config.value("server").toString());
    m_device = field(config.value("device").toString());
    m_device->setPlaceholderText(Presence::instance()->status().value("device").toString());

    serverForm->addRow("&Token:", m_token);
    serverForm->addRow("&Address:", m_server);
    serverForm->addRow("Computer &name:", m_device);
    serverForm->addRow(hintLabel("The token is the AGENT_TOKEN the Worker was given. Leave the "
                                 "computer name empty to use one made from this computer's name."));
    layout->addWidget(server);

    // Cider
    auto *cider = new QGroupBox("Cider");
    auto *ciderForm = new QFormLayout(cider);

    m_ciderToken = field(config.value("cider_token").toString());
    m_ciderToken->setEchoMode(QLineEdit::Password);
    m_ciderToken->setPlaceholderText("Optional");

    ciderForm->addRow("API t&oken:", m_ciderToken);
    ciderForm->addRow(hintLabel("From Cider > Settings > Connectivity. Without it, Cider is read "
                                "through Now Playing."));
    layout->addWidget(cider);

    // Artwork
    auto *artwork = new QGroupBox("Artwork");
    auto *artworkForm = new QFormLayout(artwork);

    QStringList countries;
    for (const QJsonValue &value : config.value("itunes_countries").toArray()) {
        countries << value.toString();
    }
    m_countries = field(countries.join(", "));

    artworkForm->addRow("iTunes &stores:", m_countries);
    artworkForm->addRow(hintLabel("Searched in this order, separated by commas. Only Apple Music "
                                  "and Spotify are looked up."));
    layout->addWidget(artwork);

    layout->addStretch();
    return page;
}

QWidget *SettingsDialog::buildDeveloperTab() {
    auto *page = new QWidget;
    auto *layout = tabLayout(page);

    // Sources: one row each, with what is using it as rows under it.
    auto *sources = new QGroupBox("Sources");
    auto *sourcesLayout = new QVBoxLayout(sources);

    m_sources = new QTreeWidget;
    m_sources->setHeaderLabels({"Source", "Status"});
    m_sources->setRootIsDecorated(false);
    m_sources->setItemsExpandable(false);
    m_sources->setSelectionMode(QAbstractItemView::NoSelection);
    m_sources->setFocusPolicy(Qt::NoFocus);
    m_sources->header()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    m_sources->header()->setSectionResizeMode(1, QHeaderView::Stretch);
    sourcesLayout->addWidget(framed(m_sources));
    layout->addWidget(sources, 1);

    // Where the settings file is, selectable so it can be copied, and a
    // button to open its folder.
    const QString path = fromRust(config_path());

    auto *pathLabel = plainLabel(path);
    pathLabel->setTextInteractionFlags(Qt::TextSelectableByMouse);
    pathLabel->setWordWrap(true);

    auto *openFolder = new QPushButton("Open &Folder");
    connect(openFolder, &QPushButton::clicked, this, [path] {
        QDesktopServices::openUrl(QUrl::fromLocalFile(QFileInfo(path).absolutePath()));
    });

    auto *fileRow = new QHBoxLayout;
    fileRow->addWidget(pathLabel, 1);
    fileRow->addWidget(openFolder, 0, Qt::AlignTop);
    layout->addLayout(fileRow);

    return page;
}

// What the app has been doing, newest first, as Event Viewer lists events.
QWidget *SettingsDialog::buildLogTab() {
    auto *page = new QWidget;
    auto *layout = tabLayout(page);

    m_log = new QTreeWidget;
    m_log->setHeaderLabels({"Time", "Event"});
    m_log->setRootIsDecorated(false);
    m_log->setAlternatingRowColors(true);
    m_log->setUniformRowHeights(true);
    m_log->setWordWrap(false);
    m_log->setTextElideMode(Qt::ElideRight);
    m_log->header()->setSectionResizeMode(0, QHeaderView::ResizeToContents);
    layout->addWidget(framed(m_log), 1);

    return page;
}

void SettingsDialog::markChanged() {
    m_buttons->button(QDialogButtonBox::Apply)->setEnabled(true);
}

void SettingsDialog::showProblem(const QString &problem) {
    m_problem->setText(problem);
    m_problem->setVisible(!problem.isEmpty());
}

bool SettingsDialog::apply() {
    // Apps that are not on this computer keep their setting.
    QJsonArray allowed = QJsonArray::fromStringList(m_hiddenAllowed);
    for (int i = 0; i < m_players->count(); i++) {
        const QListWidgetItem *item = m_players->item(i);
        if (item->checkState() == Qt::Checked) allowed.append(item->data(Qt::UserRole).toString());
    }

    QJsonArray countries;
    for (const QString &country : m_countries->text().split(',')) {
        const QString trimmed = country.trimmed();
        if (!trimmed.isEmpty()) countries.append(trimmed);
    }

    QJsonObject wanted;
    wanted["server"] = m_server->text();
    wanted["token"] = m_token->text();
    wanted["device"] = m_device->text();
    wanted["cider_token"] = m_ciderToken->text();
    wanted["itunes_countries"] = countries;
    wanted["allowed_players"] = allowed;

    QString problem = Presence::instance()->saveConfig(wanted);

    if (problem.isEmpty() && m_autostart->isChecked() != autostart_enabled()) {
        problem = fromRust(set_autostart(m_autostart->isChecked()));
    }

    showProblem(problem);
    if (!problem.isEmpty()) return false;

    m_buttons->button(QDialogButtonBox::Apply)->setEnabled(false);
    return true;
}

// A coloured square in front of each source, as Device Manager's Resources
// tab marks each resource: green when active, grey when idle, red when it
// cannot work.
static QIcon stateSquare(const QString &state) {
    QColor color(150, 150, 150);
    if (state == "current") color = QColor(40, 140, 40);
    if (state == "unknown") color = QColor(190, 40, 40);

    QPixmap pixmap(14, 14);
    pixmap.fill(Qt::transparent);

    QPainter painter(&pixmap);
    painter.setPen(QColor(60, 60, 60));
    painter.setBrush(color);
    painter.drawRect(1, 1, 11, 11);
    return QIcon(pixmap);
}

// Name, and whether it is active, idle, or why it is not working, with the
// icon of each app using it on a row under it. The app's name shows on
// hover.
void SettingsDialog::renderSources() {
    m_sources->clear();

    for (const Texts::Source &source : Texts::sources(Presence::instance()->status())) {
        auto *row = new QTreeWidgetItem(m_sources, {source.name, source.badge});
        row->setIcon(0, stateSquare(source.state));

        for (const QJsonObject &activity : source.users) {
            const QString name = activity.value("name").toString();
            const bool music = activity.value("type").toInt() == Texts::Listening;

            auto *user = new QTreeWidgetItem(row);
            user->setIcon(0, Texts::iconFor(name, music));
            user->setToolTip(0, name);
        }
    }

    m_sources->expandAll();
}

// Newest first, with 24-hour times.
void SettingsDialog::renderLog() {
    // Only when there is something new, so it does not jump back to the
    // top while being read.
    const QString json = fromRust(log_json());
    if (json == m_logJson) return;
    m_logJson = json;

    const QJsonArray lines = QJsonDocument::fromJson(json.toUtf8()).array();

    m_log->clear();

    QList<QTreeWidgetItem *> items;
    for (qsizetype i = lines.size() - 1; i >= 0; i--) {
        const QJsonObject line = lines.at(i).toObject();
        const QDateTime at = QDateTime::fromMSecsSinceEpoch(line.value("at").toInteger());
        const QString text = line.value("text").toString();

        auto *item = new QTreeWidgetItem({at.toString("HH:mm:ss"), text});
        item->setToolTip(1, text);
        items.append(item);
    }

    m_log->addTopLevelItems(items);
}
