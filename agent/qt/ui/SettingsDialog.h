#pragma once

// Settings, as a Properties dialog like linux-devmgmt's: tabs, and OK,
// Cancel and Apply. Nothing changes until OK or Apply.
//
//   General       start at login, and which apps may be shared
//   Connections   the server, Cider and artwork
//   Developer     how each source is doing, the log, and where the settings
//                 file is

#include <QDialog>
#include <QList>

class QCheckBox;
class QDialogButtonBox;
class QGridLayout;
class QLabel;
class QLineEdit;
class QPlainTextEdit;
class QTabWidget;

class SettingsDialog : public QDialog {
    Q_OBJECT

public:
    explicit SettingsDialog(int tab, QWidget *parent = nullptr);

private:
    QWidget *buildGeneralTab();
    QWidget *buildConnectionsTab();
    QWidget *buildDeveloperTab();

    // Saves what is in the dialog. False if something could not be saved,
    // and says what.
    bool apply();

    void markChanged();
    void renderSources();
    void renderLog();

    QTabWidget *m_tabs;
    QDialogButtonBox *m_buttons;
    QLabel *m_problem;

    QCheckBox *m_autostart;
    QList<QCheckBox *> m_players;

    QLineEdit *m_token;
    QLineEdit *m_server;
    QLineEdit *m_device;
    QLineEdit *m_ciderToken;
    QLineEdit *m_countries;

    QGridLayout *m_sources;
    QPlainTextEdit *m_log;
};
