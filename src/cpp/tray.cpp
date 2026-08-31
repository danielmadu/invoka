#include "tray.h"

#include <QAction>
#include <QApplication>
#include <QMenu>
#include <QObject>
#include <QSystemTrayIcon>

#include <cstdio>

namespace {

rust::Fn<void()> toggle_callback;
rust::Fn<void()> quit_callback;
rust::Fn<void()> settings_callback;

void on_toggle_triggered(bool) {
    toggle_callback();
}

void on_quit_triggered(bool) {
    quit_callback();
}

void on_settings_triggered(bool) {
    settings_callback();
}

void on_activated(QSystemTrayIcon::ActivationReason reason) {
    if (reason == QSystemTrayIcon::Trigger) {
        toggle_callback();
    }
}

}  // namespace

void invoka_app_init() {
    // QApplication (not QGuiApplication) so QWidgets such as QSystemTrayIcon
    // can exist; it fully replaces QGuiApplication for QML purposes.
    static int argc = 1;
    static char argv0[] = "invoka";
    static char* argv[] = { argv0 };
    static QApplication app(argc, argv);
    Q_UNUSED(app);
}

int invoka_app_exec() {
    if (QApplication::instance() == nullptr) {
        invoka_app_init();
    }
    return QCoreApplication::exec();
}

void invoka_tray_init(rust::Fn<void()> on_toggle, rust::Fn<void()> on_quit,
                      rust::Fn<void()> on_settings) {
    const bool available = QSystemTrayIcon::isSystemTrayAvailable();
    fprintf(stderr, "[invoka] tray available=%d\n", available ? 1 : 0);
    if (!available) {
        return;
    }

    toggle_callback = on_toggle;
    quit_callback = on_quit;
    settings_callback = on_settings;

    auto* tray = new QSystemTrayIcon(QIcon::fromTheme(QStringLiteral("system-search")));
    tray->setToolTip(QStringLiteral("Invoka"));

    auto* menu = new QMenu();
    QAction* settings_action = menu->addAction(QObject::tr("Settings"));
    settings_action->setIcon(QIcon::fromTheme(QStringLiteral("preferences-desktop")));
    QObject::connect(settings_action, &QAction::triggered, &on_settings_triggered);
    QAction* toggle_action = menu->addAction(QObject::tr("Toggle launcher"));
    QObject::connect(toggle_action, &QAction::triggered, &on_toggle_triggered);
    menu->addSeparator();
    QAction* quit_action = menu->addAction(QObject::tr("Quit"));
    QObject::connect(quit_action, &QAction::triggered, &on_quit_triggered);
    tray->setContextMenu(menu);

    QObject::connect(tray, &QSystemTrayIcon::activated, &on_activated);
    tray->show();
}
