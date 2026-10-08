// Builds the window, which is C++ with Qt Widgets (ui/), and the bridge
// between it and Rust (src/ffi.rs). cxx-qt-build finds Qt through qmake6,
// runs moc on the headers that need it, and compiles the lot.

use cxx_qt_build::CxxQtBuilder;

fn main() {
    let ui = [
        "ui/Bridge.cpp",
        "ui/Bridge.h",
        "ui/MainWindow.cpp",
        "ui/MainWindow.h",
        "ui/NowPlaying.cpp",
        "ui/NowPlaying.h",
        "ui/SettingsDialog.cpp",
        "ui/SettingsDialog.h",
        "ui/Texts.cpp",
        "ui/Tray.cpp",
        "ui/Tray.h",
    ];

    CxxQtBuilder::new()
        .file("src/ffi.rs")
        .cpp_files(ui)
        // Baked into the program: ":/icons/tray.png" and so on.
        .qrc_resources([
            "icons/tray.png",
            "icons/tray-paused.png",
            "icons/icon.png",
            "images/background.jpg",
        ])
        .qt_module("Gui")
        .qt_module("Widgets")
        .qt_module("Network")
        .build();
}
