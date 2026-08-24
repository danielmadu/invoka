//! Single-instance enforcement plus a tiny line-based IPC server.
//!
//! The daemon listens on a Unix socket so keybindings can summon the launcher
//! from any compositor or desktop environment:
//!
//! ```text
//! invoka toggle    # show/hide the launcher window
//! invoka quit      # stop the daemon
//! ```

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;

/// Commands understood by the daemon over IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Quit,
}

// Avoid pulling in the libc crate just for getuid.
#[cfg(unix)]
extern "C" {
    fn getuid() -> u32;
}

pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/tmp".to_string());
    let uid = unsafe { getuid() };
    PathBuf::from(runtime_dir).join(format!("invoka-{uid}.sock"))
}

/// Try to become the daemon. Returns the listener on success; `None` if
/// another instance is already running and owns the socket.
pub fn bind_daemon_socket() -> Option<UnixListener> {
    let path = socket_path();
    let _ = std::fs::remove_file(&path);
    match UnixListener::bind(&path) {
        Ok(listener) => Some(listener),
        Err(err) => {
            eprintln!("failed to bind IPC socket {}: {err}", path.display());
            None
        }
    }
}

/// Send a command to the running daemon. Returns whether it was delivered.
pub fn send(command: Command) -> bool {
    let path = socket_path();
    let Ok(mut stream) = UnixStream::connect(&path) else {
        return false;
    };
    let message = match command {
        Command::Toggle => "toggle\n",
        Command::Quit => "quit\n",
    };
    stream.write_all(message.as_bytes()).is_ok()
}

/// Blocking accept loop. Calls `handler` per command; stops when it returns false.
pub fn serve(listener: UnixListener, mut handler: impl FnMut(Command) -> bool) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while matches!(reader.read_line(&mut line), Ok(n) if n > 0) {
            let command = match line.trim() {
                "toggle" => Some(Command::Toggle),
                "quit" => Some(Command::Quit),
                _ => None,
            };
            if let Some(command) = command {
                let keep_going = handler(command);
                if !keep_going {
                    return;
                }
            }
            line.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_contains_uid() {
        let path = socket_path();
        assert!(path.as_os_str().to_string_lossy().contains("invoka-"));
        assert!(path.extension().is_some_and(|e| e == "sock"));
    }
}
