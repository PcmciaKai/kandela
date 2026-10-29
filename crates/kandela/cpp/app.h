#pragma once

#include <cstdint>

// Creates the QApplication, loads the QML UI and runs the event loop.
// A QApplication (rather than QGuiApplication) is needed by the
// org.kde.desktop Qt Quick style, which draws controls with Breeze.
// With `single_instance`, a second start shows the first instance's window.
std::int32_t run_app(bool start_hidden, bool single_instance);
