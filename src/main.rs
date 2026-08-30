pub mod apps;
pub mod bridge;
pub mod config;
pub mod hotkey;
pub mod ipc;
pub mod search;
pub mod theme;

use cxx_qt_lib::{QQmlApplicationEngine, QUrl};

use ipc::Command;

const MAIN_QML: &str = "qrc:/qt/qml/io/invoka/launcher/qml/Main.qml";

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("toggle") => {
            if !ipc::send(Command::Toggle) {
                run_daemon(true);
            }
        }
        Some("quit") => {
            ipc::send(Command::Quit);
        }
        Some("--version") | Some("-V") => {
            println!("invoka {}", env!("CARGO_PKG_VERSION"));
        }
        Some("debug-scan") => {
            let apps = apps::scan();
            println!("{} apps encontrados", apps.len());
            for app in apps.iter().take(15) {
                println!(
                    "  {:<28} icon={:?} terminal={} exec={}",
                    app.name,
                    app.icon.as_ref().map(|p| p.to_string_lossy()),
                    app.terminal,
                    app.exec
                );
            }
        }
        Some("--help") | Some("-h") => {
            println!("usage: invoka [toggle|quit|debug-scan]");
        }
        _ => {
            // A second bare invocation while the daemon runs toggles it.
            if !ipc::send(Command::Toggle) {
                run_daemon(false);
            }
        }
    }
}

fn run_daemon(show_immediately: bool) {
    let listener = match ipc::bind_daemon_socket() {
        Some(listener) => listener,
        None => std::process::exit(1),
    };

    // QApplication lives on the C++ side (needed for QSystemTrayIcon).
    bridge::ffi::invoka_app_init();

    let mut engine = QQmlApplicationEngine::new();

    if let Some(engine) = engine.as_mut() {
        engine.load(&QUrl::from(MAIN_QML));
    }

    // Layer shell (Hyprland/wlroots builds) or plain floating window fallback.
    let layershell_windows = bridge::ffi::invoka_layershell_setup();
    if layershell_windows == 0 {
        eprintln!("[invoka] using plain floating window (no layer shell)");
    }

    spawn_ipc_server(listener);
    hotkey::start();

    if show_immediately {
        bridge::queue_on_qt(|controller| controller.set_visible(true));
    }

    let exit_code = bridge::ffi::invoka_app_exec();
    let _ = std::fs::remove_file(ipc::socket_path());
    std::process::exit(exit_code);
}

/// Serve IPC commands until the daemon is asked to quit.
///
/// Commands arrive from foreign threads; window mutations are marshalled onto
/// the Qt event loop through the `CxxQtThread` captured by `bootstrap()`.
fn spawn_ipc_server(listener: std::os::unix::net::UnixListener) {
    std::thread::spawn(move || {
        ipc::serve(listener, |command| match command {
            Command::Toggle => {
                bridge::toggle_window();
                true
            }
            Command::Quit => {
                bridge::queue_on_qt(|_controller| {
                    std::process::exit(0);
                });
                // Qt event loop may not be reachable (e.g. failed QML load);
                // quit unconditionally anyway.
                std::process::exit(0);
            }
        });
    });
}
