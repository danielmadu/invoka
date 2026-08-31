//! Single-instance enforcement plus a tiny line-based IPC server.
//!
//! The daemon listens on a Unix socket (Linux) or a named pipe (Windows) so
//! keybindings can summon the launcher from any compositor or desktop
//! environment:
//!
//! ```text
//! invoka toggle    # show/hide the launcher window
//! invoka quit      # stop the daemon
//! ```

/// Commands understood by the daemon over IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Toggle,
    Quit,
}

/// Parse a wire command line.
pub fn parse_command(line: &str) -> Option<Command> {
    match line.trim() {
        "toggle" => Some(Command::Toggle),
        "quit" => Some(Command::Quit),
        _ => None,
    }
}

#[cfg(unix)]
mod imp {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;

    use super::{parse_command, Command};

    // Avoid pulling in the libc crate just for getuid.
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
        write_command(&mut stream, command)
    }

    fn write_command(stream: &mut UnixStream, command: Command) -> bool {
        let message = match command {
            Command::Toggle => "toggle\n",
            Command::Quit => "quit\n",
        };
        stream.write_all(message.as_bytes()).is_ok()
    }

    /// Blocking accept loop. Calls `handler` per command; stops when it
    /// returns false.
    pub fn serve(listener: UnixListener, mut handler: impl FnMut(Command) -> bool) {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            while matches!(reader.read_line(&mut line), Ok(n) if n > 0) {
                if let Some(command) = parse_command(&line) {
                    if !handler(command) {
                        return;
                    }
                }
                line.clear();
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn socket_path_for_test() -> PathBuf {
        socket_path()
    }
}

#[cfg(windows)]
mod imp {
    use std::io::Error;

    use windows_sys::Win32::Foundation::{
        CloseHandle, ERROR_PIPE_CONNECTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_SHARE_NONE, FlushFileBuffers,
        OPEN_EXISTING, PIPE_ACCESS_INBOUND, ReadFile, WriteFile,
    };
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
    };

    use super::{parse_command, Command};

    /// Named pipe for the per-user daemon (`\\.\pipe\invoka-<username>`).
    pub fn socket_path() -> String {
        format!(r"\\.\pipe\invoka-{}", user_id())
    }

    /// Per-user pipe suffix: sanitized `%USERNAME%`.
    pub fn user_id() -> String {
        let raw = std::env::var("USERNAME").unwrap_or_else(|_| "default".to_string());
        let sanitized: String = raw
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
            .collect();
        if sanitized.is_empty() {
            "default".to_string()
        } else {
            sanitized
        }
    }

    fn to_wide(value: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// Create one inbound pipe server instance. `first` adds
    /// `FILE_FLAG_FIRST_PIPE_INSTANCE` so a second daemon fails to bind.
    fn create_instance(first: bool) -> Option<HANDLE> {
        let name = to_wide(&socket_path());
        let access = PIPE_ACCESS_INBOUND | (u32::from(first) * FILE_FLAG_FIRST_PIPE_INSTANCE);
        let handle = unsafe {
            CreateNamedPipeW(
                name.as_ptr(),
                access,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                1024,
                1024,
                0,
                std::ptr::null(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            None
        } else {
            Some(handle)
        }
    }

    /// Try to become the daemon. Returns the first pipe instance on success;
    /// `None` if another instance is already running (or the pipe could not
    /// be created).
    pub fn bind_daemon_socket() -> Option<HANDLE> {
        match create_instance(true) {
            Some(handle) => Some(handle),
            None => {
                let err = Error::from_raw_os_error(unsafe { GetLastError() } as i32);
                eprintln!("failed to bind IPC pipe {}: {err}", socket_path());
                None
            }
        }
    }

    /// Send a command to the running daemon. Returns whether it was delivered.
    pub fn send(command: Command) -> bool {
        let name = to_wide(&socket_path());
        let handle = unsafe {
            CreateFileW(
                name.as_ptr(),
                0x4000_0000, // GENERIC_WRITE
                FILE_SHARE_NONE,
                std::ptr::null(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE || handle.is_null() {
            return false;
        }

        let message: &[u8] = match command {
            Command::Toggle => b"toggle\n",
            Command::Quit => b"quit\n",
        };
        let mut written = 0u32;
        let ok = unsafe {
            WriteFile(
                handle,
                message.as_ptr(),
                message.len() as u32,
                &mut written,
                std::ptr::null_mut(),
            )
        } != 0;
        unsafe { CloseHandle(handle) };
        ok
    }

    /// Raw `HANDLE` is `!Send`; the pipe instance is only ever handed to the
    /// single dedicated server thread, so the newtype promise is sound.
    pub struct IpcListener(pub HANDLE);

    unsafe impl Send for IpcListener {}

    /// Blocking accept loop. Calls `handler` per command; stops when it
    /// returns false.
    pub fn serve(listener: IpcListener, mut handler: impl FnMut(Command) -> bool) {
        serve_loop(listener.0, &mut handler)
    }

    fn serve_loop(
        listener: HANDLE,
        handler: &mut impl FnMut(Command) -> bool,
    ) {
        let mut first = true;
        let mut quit = false;
        while !quit {            // Reuse the instance created during bind for the first client;
            // afterwards create a fresh one per iteration.
            let handle = if first {
                first = false;
                Some(listener)
            } else {
                create_instance(false)
            };
            let Some(handle) = handle else {
                eprintln!("[invoka] failed to create IPC pipe instance");
                return;
            };

            // A client that connected between CreateNamedPipeW and
            // ConnectNamedPipe makes the latter fail with
            // ERROR_PIPE_CONNECTED; both outcomes are fine.
            let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) };
            if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
                let err = Error::from_raw_os_error(unsafe { GetLastError() } as i32);
                eprintln!("[invoka] IPC pipe connect failed: {err}");
                unsafe { CloseHandle(handle) };
                continue;
            }

            quit = read_commands(handle, &mut *handler);

            unsafe {
                FlushFileBuffers(handle);
                DisconnectNamedPipe(handle);
                CloseHandle(handle);
            }
        }
    }

    /// Read newline-delimited commands until the client disconnects or the
    /// handler asks to stop (true returned = stop the whole server).
    fn read_commands(handle: HANDLE, handler: &mut impl FnMut(Command) -> bool) -> bool {
        let mut buffer = [0u8; 512];
        let mut pending: Vec<u8> = Vec::new();
        loop {
            let mut read = 0u32;
            let ok = unsafe {
                ReadFile(
                    handle,
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    &mut read,
                    std::ptr::null_mut(),
                )
            };
            if ok == 0 || read == 0 {
                return false;
            }
            pending.extend_from_slice(&buffer[..read as usize]);
            while let Some(pos) = pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = pending.drain(..=pos).collect();
                let line = String::from_utf8_lossy(&line[..line.len() - 1]).into_owned();
                if let Some(command) = parse_command(&line) {
                    if !handler(command) {
                        return true;
                    }
                }
            }
        }
    }
}

pub use imp::*;

#[cfg(unix)]
/// Listener type handed to [`serve`] (Unix socket).
pub type IpcListener = std::os::unix::net::UnixListener;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_commands() {
        assert_eq!(parse_command("toggle"), Some(Command::Toggle));
        assert_eq!(parse_command(" quit \n"), Some(Command::Quit));
        assert_eq!(parse_command("bogus"), None);
        assert_eq!(parse_command(""), None);
    }
}

#[cfg(unix)]
#[cfg(test)]
mod unix_tests {
    use super::imp::socket_path_for_test;

    #[test]
    fn socket_path_contains_uid() {
        let path = socket_path_for_test();
        assert!(path.as_os_str().to_string_lossy().contains("invoka-"));
        assert!(path.extension().is_some_and(|e| e == "sock"));
    }
}

#[cfg(windows)]
#[cfg(test)]
mod windows_tests {
    use super::imp::user_id;
    use super::*;

    #[test]
    fn pipe_name_is_namespaced_and_per_user() {
        let path = imp::socket_path();
        assert!(path.starts_with(r"\\.\pipe\invoka-"));
        assert!(user_id()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
    }
}
