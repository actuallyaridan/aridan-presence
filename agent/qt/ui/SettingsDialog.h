#pragma once

// Settings, as a Windows 7 Properties dialog: tabs, and OK, Cancel and
// Apply. Nothing changes until OK or Apply. Its width, margins and the thin
// frames round its lists follow linux-devmgmt's Properties dialog.
//
//   General       start at login, and which apps may be shared
//   Connections   the server, Cider and artwork
//   Developer     how each source is doing, and where the settings file is
//   Log           what the app has been doing

#include <QDialog>
#include <QString>
#include <QStringList>

class QCheckBox;
class QDialogButtonBox;
class QLabel;
class QLineEdit;
class QListWidget;
class QTabWidget;
class QTreeWidget;
class QWidget;

class SettingsDialog : public QDialog {
    Q_OBJECT

public:
    enum Tab { General, Connections, Developer, Log };

    explicit SettingsDialog(Tab tab, QWidget *parent = nullptr);

    void showTab(Tab tab);

protected:
    void showEvent(QShowEvent *event) override;

private:
    QWidget *buildGeneralTab();
    QWidget *buildConnectionsTab();
    QWidget *buildDeveloperTab();
    QWidget *buildLogTab();

    // Saves what is in the dialog. False if something could not be saved,
    // and says what.
    bool apply();

    void markChanged();
    void showProblem(const QString &problem);
    void renderSources();
    void renderLog();

    QTabWidget *m_tabs;
    QDialogButtonBox *m_buttons;
    QLabel *m_problem;
    bool m_sized = false;

    QCheckBox *m_autostart;
    QListWidget *m_players;
    QStringList m_hiddenAllowed;

    QLineEdit *m_token;
    QLineEdit *m_server;
    QLineEdit *m_device;
    QLineEdit *m_ciderToken;
    QLineEdit *m_countries;

    QTreeWidget *m_sources;
    QTreeWidget *m_log;
    QString m_logJson;
};
