use {
    crossbeam_channel::Sender,
    std::io::Write,
    std::os::unix::net::{UnixListener, UnixStream},
    std::path::PathBuf,
    std::thread,
};

pub fn socket_path() -> PathBuf {
    std::env::var("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share").join("clipwise"))
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
        })
        .join("clipwise.sock")
}

/// Returns true if a daemon was already running and has been signalled.
/// Returns false if no daemon is running (stale socket removed if present).
pub fn try_signal_existing() -> bool {
    let path = socket_path();
    match UnixStream::connect(&path) {
        Ok(mut stream) => {
            let _ = stream.write_all(b"show\n");
            true
        }
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            false
        }
    }
}

/// Binds the Unix socket and spawns a thread that sends `()` on `show_tx`
/// and calls `ctx.request_repaint()` for every incoming connection.
/// Must be called after `socket_path()` is free (i.e., after `try_signal_existing()` returned false).
pub fn start_listener(show_tx: Sender<()>, ctx: egui::Context) {
    let path = socket_path();
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("clipwise: failed to bind IPC socket: {e}");
            return;
        }
    };
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(_) => {
                    let _ = show_tx.send(());
                    ctx.request_repaint();
                }
                Err(e) => {
                    eprintln!("clipwise: IPC accept error: {e}");
                    break;
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn socket_path_filename_is_correct() {
        let path = socket_path();
        assert_eq!(path.file_name().unwrap(), "clipwise.sock");
    }

    #[test]
    fn socket_path_uses_xdg_runtime_dir() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Set a known value, check it appears in the path
        std::env::set_var("XDG_RUNTIME_DIR", "/tmp/clipwise_test_xdg");
        let path = socket_path();
        assert_eq!(
            path,
            PathBuf::from("/tmp/clipwise_test_xdg/clipwise.sock")
        );
        // Clean up so other tests are not affected
        std::env::remove_var("XDG_RUNTIME_DIR");
    }

    #[test]
    fn ipc_roundtrip_listener_receives_signal() {
        use std::time::Duration;

        // Use a dedicated temp socket so this test is isolated
        let test_sock = std::env::temp_dir().join("clipwise_ipc_roundtrip_test.sock");
        let _ = std::fs::remove_file(&test_sock);

        let (tx, rx) = crossbeam_channel::unbounded::<()>();

        // Bind the listener manually on the test socket
        let listener = UnixListener::bind(&test_sock).expect("bind test socket");
        thread::spawn(move || {
            for stream in listener.incoming() {
                if stream.is_ok() {
                    let _ = tx.send(());
                }
            }
        });

        // Connect as client and write the show signal
        let mut stream = UnixStream::connect(&test_sock).expect("connect test socket");
        stream.write_all(b"show\n").expect("write show signal");
        drop(stream);

        // Listener should deliver () within 1 second
        let result = rx.recv_timeout(Duration::from_secs(1));
        assert!(result.is_ok(), "listener should forward the show signal");

        let _ = std::fs::remove_file(&test_sock);
    }
}
