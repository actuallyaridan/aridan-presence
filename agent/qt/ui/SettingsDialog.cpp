#include "SettingsDialog.h"
#include "Bridge.h"
#include "IconHelper.h"
#include "Texts.h"

// Rust's functions: the log, start at login, where the settings live.
#include "aridan-presence-qt/src/ffi.cxxqt.h"

#include <QApplication>
#include <QCheckBox>
#include <QDateTime>
#include <QDialogButtonBox>
#include <QFontDatabase>
#include <QFormLayout>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QLabel>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QTabWidget>
#include <QVBoxLayout>

namespace {

QString fromRust(const rust::String &text) {
    return QString::fromUtf8(text.data(), static_cast<qsizetype>(text.size()));
}

QLabel *plainLabel(const QString &text) {
    auto *label = new QLabel;
    label->setTextFormat(Qt::PlainText);
    label->setText(text);
    return label;
}

// A line of explanation under a group's fields, greyed like a hint.
QLabel *hintLabel(const QString &text) {
    auto *label = plainLabel(text);
    label->setWordWrap(true);
    label->setEnabled(false);
    return label;
}

// The app's icon, name and version across the top of a tab, as
// linux-devmgmt's Properties dialog shows the device.
QWidget *appHeader() {
    auto *row = new QWidget;
    auto *layout = new QHBoxLayout(row);
    layout->setContentsMargins(0, 0, 0, 8);

    auto *icon = new QLabel;
    icon->setPixmap(appIcon().pixmap(32, 32));
    icon->setFixedSize(40, 40);
    icon->setAlignment(Qt::AlignTop);

    auto *label = plainLabel("aridan-presence " + QApplication::applicationVersion());

    layout->addWidget(icon);
    layout->addWidget(label, 1);
    return row;
}

QColor sourceColor(const QString &state) {
    if (state == "current") return QColor("#23692f");
    if (state == "unknown") return QColor("#ba0a00");
    return QColor();
}

} // namespace

SettingsDialog::SettingsDialog(int tab, QWidget *parent) : QDialog(parent) {
    setWindowTitle("aridan-presence Settings");
    setWindowFlags(Qt::Dialog | Qt::CustomizeWindowHint | Qt::WindowTitleHint | Qt::WindowCloseButtonHint);

    auto *layout = new QVBoxLayout(this);
    layout->setContentsMargins(8, 8, 8, 8);

    m_tabs = new QTabWidget;
    m_tabs->addTab(buildGeneralTab(), "General");
    m_tabs->addTab(buildConnectionsTab(), "Connections");
    m_tabs->addTab(buildDeveloperTab(), "Developer");
    m_tabs->setCurrentIndex(tab);
    layout->addWidget(m_tabs);

    // A problem with the settings file at startup, or with saving.
    m_problem = new QLabel;
    m_problem->setWordWrap(true);
    m_problem->setStyleSheet("color: #ba0a00;");
    m_problem->setText(Presence::instance()->status().value("config_error").toString());
    m_problem->setVisible(!m_problem->text().isEmpty());
    layout->addWidget(m_problem);

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

    resize(440, 520);
}

QWidget *SettingsDialog::buildGeneralTab() {
    auto *page = new QWidget;
    auto *layout = new QVBoxLayout(page);

    layout->addWidget(appHeader());

    m_autostart = new QCheckBox("Start when I log in");
    m_autostart->setChecked(autostart_enabled());
    connect(m_autostart, &QCheckBox::toggled, this, &SettingsDialog::markChanged);
    layout->addWidget(m_autostart);

    auto *apps = new QGroupBox("Apps that may be shared");
    auto *appsLayout = new QVBoxLayout(apps);

    const QJsonObject config = Presence::instance()->config();
    const QJsonObject status = Presence::instance()->status();

    for (const Texts::Player &player : Texts::playerList(config, status)) {
        auto *box = new QCheckBox(player.name);
        box->setChecked(player.allowed);
        box->setProperty("key", player.key);
        connect(box, &QCheckBox::toggled, this, &SettingsDialog::markChanged);

        m_players.append(box);
        appsLayout->addWidget(box);
    }

    appsLayout->addSpacing(4);
    appsLayout->addWidget(hintLabel("Anything else playing - a video call, a training video - is never shared."));

    layout->addWidget(apps);
    layout->addStretch();
    return page;
}

QWidget *SettingsDialog::buildConnectionsTab() {
    auto *page = new QWidget;
    auto *layout = new QVBoxLayout(page);

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
    auto *layout = new QVBoxLayout(page);

    auto *sources = new QGroupBox("Sources");
    m_sources = new QGridLayout(sources);
    m_sources->setColumnStretch(0, 1);
    layout->addWidget(sources);

    auto *log = new QGroupBox("Log");
    auto *logLayout = new QVBoxLayout(log);

    m_log = new QPlainTextEdit;
    m_log->setReadOnly(true);
    m_log->setFont(QFontDatabase::systemFont(QFontDatabase::FixedFont));
    logLayout->addWidget(m_log);
    layout->addWidget(log, 1);

    // Selectable, so the path can be copied.
    auto *path = plainLabel(fromRust(config_path()));
    path->setTextInteractionFlags(Qt::TextSelectableByMouse);
    path->setEnabled(false);
    layout->addWidget(path);

    return page;
}

void SettingsDialog::markChanged() {
    m_buttons->button(QDialogButtonBox::Apply)->setEnabled(true);
}

bool SettingsDialog::apply() {
    QJsonArray allowed;
    for (QCheckBox *box : m_players) {
        if (box->isChecked()) allowed.append(box->property("key").toString());
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

    m_problem->setText(problem);
    m_problem->setVisible(!problem.isEmpty());

    if (!problem.isEmpty()) return false;

    m_buttons->button(QDialogButtonBox::Apply)->setEnabled(false);
    return true;
}

// Name on the left and whether it is active on the right, with the icons
// of whatever is using it under the name.
void SettingsDialog::renderSources() {
    while (QLayoutItem *item = m_sources->takeAt(0)) {
        delete item->widget();
        delete item;
    }

    int row = 0;
    for (const Texts::Source &source : Texts::sources(Presence::instance()->status())) {
        auto *name = plainLabel(source.name);
        QFont bold = name->font();
        bold.setBold(true);
        name->setFont(bold);

        auto *badge = plainLabel(source.badge);
        const QColor color = sourceColor(source.state);
        if (color.isValid()) {
            badge->setStyleSheet("color: " + color.name() + ";");
        } else {
            badge->setEnabled(false);
        }

        m_sources->addWidget(name, row, 0);
        m_sources->addWidget(badge, row, 1, Qt::AlignRight);

        if (!source.users.isEmpty()) {
            auto *icons = new QWidget;
            auto *iconsLayout = new QHBoxLayout(icons);
            iconsLayout->setContentsMargins(0, 2, 0, 0);
            iconsLayout->setSpacing(6);

            for (const QJsonObject &activity : source.users) {
                const QString activityName = activity.value("name").toString();
                const bool music = activity.value("type").toInt() == Texts::Listening;

                auto *icon = new QLabel;
                icon->setPixmap(Texts::iconFor(activityName, music).pixmap(16, 16));
                icon->setToolTip(activityName);
                iconsLayout->addWidget(icon);
            }
            iconsLayout->addStretch();

            m_sources->addWidget(icons, row + 1, 0, 1, 2);
        }

        m_sources->setRowMinimumHeight(row + 2, 6);
        row += 3;
    }
}

// Newest first, with 24-hour times.
void SettingsDialog::renderLog() {
    const QJsonArray lines = QJsonDocument::fromJson(fromRust(log_json()).toUtf8()).array();

    QStringList out;
    for (qsizetype i = lines.size() - 1; i >= 0; i--) {
        const QJsonObject line = lines.at(i).toObject();
        const QDateTime at = QDateTime::fromMSecsSinceEpoch(line.value("at").toInteger());
        out << at.toString("HH:mm:ss") + "  " + line.value("text").toString();
    }

    m_log->setPlainText(out.join('\n'));
}
