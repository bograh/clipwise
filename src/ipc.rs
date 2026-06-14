use crossbeam_channel::Sender;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_path_filename_is_correct() {
        let path = socket_path();
        assert_eq!(path.file_name().unwrap(), "clipwise.sock");
    }

    #[test]
    fn socket_path_uses_xdg_runtime_dir() {
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
}
