#pragma once

// Icons from the desktop's icon theme, trying each name in turn - the same
// helper as linux-devmgmt and linux-taskmgr, since themes name things
// differently.

#include <QIcon>
#include <QString>
#include <initializer_list>

inline QIcon tryIconName(const QString &name) {
    QIcon icon = QIcon::fromTheme(name);
    if (!icon.isNull()) return icon;
    return QIcon::fromTheme(name + QStringLiteral("-symbolic"));
}

// The first of `names` the theme has, or an empty icon.
inline QIcon themeIcon(std::initializer_list<const char *> names) {
    for (const char *n : names) {
        QIcon icon = tryIconName(QString::fromLatin1(n));
        if (!icon.isNull()) return icon;
    }
    return QIcon();
}

// The app's own icon, baked into the program.
inline QIcon appIcon() {
    return QIcon(QStringLiteral(":/icons/icon.png"));
}
