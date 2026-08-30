use cxx_qt_build::{CxxQtBuilder, QmlModule};

fn main() {
    // When the `layer-shell` feature is on, locate LayerShellQt via
    // pkg-config. A successful probe emits the link/search flags for cargo
    // and hands us the include paths used below to compile the glue with
    // INVOKA_LAYER_SHELL defined; a failure keeps the stub build.
    #[cfg(feature = "layer-shell")]
    let layer_shell_includes = {
        let probe = pkg_config::Config::new()
            .atleast_version("5.0")
            .probe("layer-shell-qt")
            .or_else(|_| pkg_config::Config::new().atleast_version("5.0").probe("LayerShellQt"));
        match probe {
            Ok(lib) => lib.include_paths,
            Err(err) => {
                println!(
                    "cargo:warning=invoka: layer-shell feature enabled but LayerShellQt was not found ({err}); building fallback"
                );
                Vec::new()
            }
        }
    };

    let builder =
        CxxQtBuilder::new_qml_module(QmlModule::new("io.invoka.launcher").qml_file("qml/Main.qml"))
            .files(["src/bridge/mod.rs"])
            // QSystemTrayIcon lives in QtWidgets
            .qt_module("Widgets")
            .cpp_file("src/cpp/tray.cpp")
            .cpp_file("src/cpp/layershell.cpp")
            .include_dir("src/cpp");

    #[cfg(feature = "layer-shell")]
    let builder = unsafe {
        builder.cc_builder(move |cc| {
            if !layer_shell_includes.is_empty() {
                cc.define("INVOKA_LAYER_SHELL", None);
                for path in &layer_shell_includes {
                    cc.include(path);
                }
            }
        })
    };

    builder.build();
}
