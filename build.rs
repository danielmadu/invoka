use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    CxxQtBuilder::new_qml_module(QmlModule::new("io.invoka.launcher").qml_file("qml/Main.qml"))
        .files(["src/bridge/mod.rs"])
        // QSystemTrayIcon lives in QtWidgets
        .qt_module("Widgets")
        .cpp_file("src/cpp/tray.cpp")
        .include_dir("src/cpp")
        .build();
}
