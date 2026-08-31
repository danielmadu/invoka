#pragma once

#include "rust/cxx.h"

// Construct the QApplication instance (must run before any QWidget/QML code).
void invoka_app_init();

// Run the Qt event loop; blocks until quit(). Returns the exit code.
int invoka_app_exec();

// Create the system tray icon with a Settings/Toggle/Quit menu. No-op when
// the desktop environment has no tray (e.g. plain GNOME without AppIndicator).
void invoka_tray_init(rust::Fn<void()> on_toggle, rust::Fn<void()> on_quit,
                      rust::Fn<void()> on_settings);
