# Global Hotkey Daemon Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Clipwise run as a persistent hidden daemon that shows/hides its window in response to Unix socket signals, so the user's WM can bind Super+V to `~/.local/bin/clipwise`.

**Architecture:** First invocation binds a Unix socket and starts hidden; subsequent invocations (WM hotkey) write `"show\n"` to the socket and exit. The daemon's IPC listener thread wakes the egui event loop immediately via `ctx.request_repaint()`, so the window appears without the 500 ms poll delay. Dismiss replaces `ViewportCommand::Close` with `ViewportCommand::Visible(false)`.

**Tech Stack:** Rust stable, eframe 0.28 / egui 0.28, `std::os::unix::net` (no new crates).

---

## File Map

| Action | Path | Responsibility |
|--------|------|----------------|
| **Create** | `src/ipc.rs` | Socket path, single-instance check, listener thread |
| **Modify** | `src/main.rs` | Single-instance guard, hidden viewport, wire channels |
| **Modify** | `src/app.rs` | `show_receiver` field, drain loop in `update()`, start listener |
| **Modify** | `src/ui.rs` | Replace `Close` with `Visible(false)` in two locations |
| **Modify** | `install.sh` | Background launch, print WM hotkey instructions |

---

## Task 1: Create `src/ipc.rs` — socket path and unit tests

**Files:**
- Create: `src/ipc.rs`

- [ ] **Step 1.1: Create the file with `socket_path()` and its tests**

```rust
// src/ipc.rs
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
```

- [ ] **Step 1.2: Run the tests to verify they pass**

```bash
cargo test ipc::tests -- --nocapture
```

Expected: both tests pass (`socket_path_filename_is_correct`, `socket_path_uses_xdg_runtime_dir`).

- [ ] **Step 1.3: Commit**

```bash
git add src/ipc.rs
git commit -m "feat: add ipc module with socket_path()"
```

---

## Task 2: Add `try_signal_existing()` and `start_listener()` with integration test

**Files:**
- Modify: `src/ipc.rs`

- [ ] **Step 2.1: Write the integration test first (append to the `tests` module in `src/ipc.rs`)**

```rust
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
```

- [ ] **Step 2.2: Run the test to verify it fails (function not yet implemented)**

```bash
cargo test ipc_roundtrip -- --nocapture
```

Expected: compile error or test failure — `tx` is unused, functions don't exist yet.

- [ ] **Step 2.3: Implement `try_signal_existing()` and `start_listener()` — append after `socket_path()` in `src/ipc.rs`**

```rust
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
            if stream.is_ok() {
                let _ = show_tx.send(());
                ctx.request_repaint();
            }
        }
    });
}
```

Note: `start_listener` takes `egui::Context` (which is `Clone + Send`) so it can immediately wake the event loop — the window appears without waiting for the 500 ms poll cycle.

- [ ] **Step 2.4: Add the egui import at the top of `src/ipc.rs`**

The full import block at the top of the file should be:

```rust
use crossbeam_channel::Sender;
use std::io::Write;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::thread;
```

`egui::Context` is referenced as a fully-qualified path in the function signature — no additional `use` is needed at the top since `egui` is already a crate dependency.

- [ ] **Step 2.5: Run all ipc tests**

```bash
cargo test ipc -- --nocapture
```

Expected: all three tests pass (`socket_path_filename_is_correct`, `socket_path_uses_xdg_runtime_dir`, `ipc_roundtrip_listener_receives_signal`).

- [ ] **Step 2.6: Commit**

```bash
git add src/ipc.rs
git commit -m "feat: add try_signal_existing() and start_listener() to ipc module"
```

---

## Task 3: Update `src/main.rs` — single-instance guard, hidden viewport, wire channels

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 3.1: Replace the entire file contents**

```rust
mod app;
mod clipboard;
mod ipc;
mod storage;
mod theme;
mod ui;

use app::ClipwiseApp;
use storage::Storage;

fn main() {
    // If a daemon is already running, signal it to show and exit.
    if ipc::try_signal_existing() {
        return;
    }

    let storage = Storage::open()
        .expect("Failed to open clipboard storage at ~/.local/share/clipwise/db");
    let items = storage.load_all().expect("Failed to load clipboard history");

    let (clip_tx, clip_rx) = crossbeam_channel::unbounded();
    clipboard::start_watcher(clip_tx);

    // show_tx goes to the IPC listener (started inside ClipwiseApp::new where
    // egui::Context is available); show_rx is drained in update().
    let (show_tx, show_rx) = crossbeam_channel::unbounded();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 520.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_visible(false),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Clipwise",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ClipwiseApp::new(
                cc, items, storage, clip_rx, show_tx, show_rx,
            )))
        }),
    )
    .expect("Failed to launch Clipwise");
}
```

- [ ] **Step 3.2: Check it compiles (app.rs signature will mismatch — expected)**

```bash
cargo check 2>&1 | head -20
```

Expected: errors about `ClipwiseApp::new` argument count mismatch. That's correct — app.rs hasn't been updated yet.

- [ ] **Step 3.3: Commit the partial change**

```bash
git add src/main.rs
git commit -m "feat: add single-instance guard and hidden viewport in main.rs"
```

---

## Task 4: Update `src/app.rs` — `show_receiver` field and listener startup

**Files:**
- Modify: `src/app.rs`

- [ ] **Step 4.1: Replace the entire file contents**

```rust
use crate::clipboard::ClipboardItem;
use crate::storage::Storage;
use crate::ui;
use chrono::Utc;
use crossbeam_channel::Receiver;

const MAX_UNPINNED: usize = 500;

pub struct ClipwiseApp {
    pub items: Vec<ClipboardItem>,
    pub search_query: String,
    pub storage: Storage,
    pub receiver: Receiver<ClipboardItem>,
    pub show_receiver: Receiver<()>,
    pub confirm_delete: Option<String>,
    pub selected_index: usize,
    pub focus_requested: bool,
}

impl ClipwiseApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        items: Vec<ClipboardItem>,
        storage: Storage,
        receiver: Receiver<ClipboardItem>,
        show_tx: crossbeam_channel::Sender<()>,
        show_receiver: Receiver<()>,
    ) -> Self {
        crate::theme::setup_visuals(&cc.egui_ctx);
        crate::ipc::start_listener(show_tx, cc.egui_ctx.clone());
        Self {
            items,
            search_query: String::new(),
            storage,
            receiver,
            show_receiver,
            confirm_delete: None,
            selected_index: 0,
            focus_requested: false,
        }
    }
}

pub fn sort_items(items: &mut Vec<ClipboardItem>) {
    items.sort_by(|a, b| match (a.pinned, b.pinned) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.copied_at.cmp(&a.copied_at),
    });
}

impl eframe::App for ClipwiseApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain show signals from the IPC listener.
        while let Ok(()) = self.show_receiver.try_recv() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.search_query.clear();
            self.focus_requested = false;
        }

        while let Ok(new_item) = self.receiver.try_recv() {
            if let Some(pos) = self.items.iter().position(|i| i.content == new_item.content) {
                let mut existing = self.items.remove(pos);
                existing.copied_at = Utc::now();
                self.items.insert(0, existing);
            } else {
                self.items.insert(0, new_item);
                let unpinned_count = self.items.iter().filter(|i| !i.pinned).count();
                if unpinned_count > MAX_UNPINNED {
                    if let Some(pos) = self.items.iter().rposition(|i| !i.pinned) {
                        self.items.remove(pos);
                    }
                }
            }
            sort_items(&mut self.items);
            let _ = self.storage.save_all(&self.items);
        }

        ui::render(ctx, self);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}
```

- [ ] **Step 4.2: Verify it compiles**

```bash
cargo check 2>&1 | head -20
```

Expected: only errors in `src/ui.rs` (still references `ViewportCommand::Close`). app.rs and main.rs should be clean.

- [ ] **Step 4.3: Commit**

```bash
git add src/app.rs
git commit -m "feat: wire show_receiver and IPC listener startup into ClipwiseApp"
```

---

## Task 5: Update `src/ui.rs` — hide instead of close

**Files:**
- Modify: `src/ui.rs`

There are exactly two `ViewportCommand::Close` calls. Both get replaced with `Visible(false)` + state reset.

- [ ] **Step 5.1: Replace the Escape handler (lines 219–225 in the original)**

Find:
```rust
    if do_escape {
        if !app.search_query.is_empty() {
            app.search_query.clear();
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
```

Replace with:
```rust
    if do_escape {
        if !app.search_query.is_empty() {
            app.search_query.clear();
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            app.focus_requested = false;
        }
    }
```

- [ ] **Step 5.2: Replace the Enter handler (lines 235–245 in the original)**

Find:
```rust
    if do_enter {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get(item_idx) {
                let content = item.content.clone();
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(content);
                }
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
```

Replace with:
```rust
    if do_enter {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get(item_idx) {
                let content = item.content.clone();
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(content);
                }
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        app.focus_requested = false;
    }
```

- [ ] **Step 5.3: Verify clean build**

```bash
cargo check 2>&1
```

Expected: no errors, no warnings about unused imports.

- [ ] **Step 5.4: Run all tests to confirm nothing regressed**

```bash
cargo test -- --nocapture
```

Expected: all three ipc tests pass.

- [ ] **Step 5.5: Commit**

```bash
git add src/ui.rs
git commit -m "feat: hide window on dismiss instead of closing the process"
```

---

## Task 6: Update `install.sh` — background launch and WM hotkey instructions

**Files:**
- Modify: `install.sh`

- [ ] **Step 6.1: Replace the final section (section 6) of `install.sh`**

Find:
```bash
# ── 6. Run ────────────────────────────────────────────────────────────────────

success "All done! Launching Clipwise..."
exec "$INSTALL_DIR/clipwise"
```

Replace with:
```bash
# ── 6. Run ────────────────────────────────────────────────────────────────────

# Kill any running instance so we start fresh with the new binary.
pkill -x clipwise 2>/dev/null || true
sleep 0.3

info "Starting Clipwise daemon in background..."
"$INSTALL_DIR/clipwise" &
success "Clipwise is running (window hidden)."

# ── 7. Hotkey instructions ────────────────────────────────────────────────────

echo ""
success "Bind Super+V in your window manager to: $INSTALL_DIR/clipwise"
echo ""
info "  i3 / i3-gaps — add to ~/.config/i3/config:"
info "    bindsym \$mod+v exec --no-startup-id $INSTALL_DIR/clipwise"
echo ""
info "  Sway — add to ~/.config/sway/config:"
info "    bindsym \$mod+v exec $INSTALL_DIR/clipwise"
echo ""
info "  GNOME — Settings → Keyboard → Custom Shortcuts:"
info "    Command: $INSTALL_DIR/clipwise    Shortcut: Super+V"
echo ""
info "  KDE — System Settings → Shortcuts → Custom Shortcuts:"
info "    Trigger: Super+V    Action: $INSTALL_DIR/clipwise"
echo ""
info "The autostart entry at $DESKTOP_FILE will start the daemon at login."
```

- [ ] **Step 6.2: Verify the script is valid shell**

```bash
bash -n install.sh && echo "syntax ok"
```

Expected: `syntax ok`

- [ ] **Step 6.3: Commit**

```bash
git add install.sh
git commit -m "feat: launch clipwise as background daemon and print WM hotkey instructions"
```

---

## Task 7: Build release binary and smoke test

- [ ] **Step 7.1: Build release binary**

```bash
cargo build --release 2>&1 | tail -5
```

Expected: `Finished release [optimized] target(s) in ...`

- [ ] **Step 7.2: Confirm no existing socket, then start the daemon**

```bash
rm -f "${XDG_RUNTIME_DIR:-$HOME/.local/share/clipwise}/clipwise.sock"
./target/release/clipwise &
DAEMON_PID=$!
sleep 0.5
echo "Daemon PID: $DAEMON_PID"
ls -la "${XDG_RUNTIME_DIR:-$HOME/.local/share/clipwise}/clipwise.sock" && echo "socket exists"
```

Expected: socket file appears, no window visible yet.

- [ ] **Step 7.3: Signal the daemon (simulates WM hotkey)**

```bash
./target/release/clipwise
```

Expected: this second invocation exits immediately (client role); the daemon's window becomes visible.

- [ ] **Step 7.4: Verify client exits instantly and socket persists**

```bash
ls -la "${XDG_RUNTIME_DIR:-$HOME/.local/share/clipwise}/clipwise.sock" && echo "socket still exists"
kill $DAEMON_PID
```

Expected: socket still present (daemon is still running); `kill` stops the daemon.

- [ ] **Step 7.5: Final commit if any files were adjusted during smoke test**

```bash
git status
# If clean:
echo "No fixups needed."
# If dirty:
git add -p && git commit -m "fix: smoke test fixups"
```

---

## Self-Review

**Spec coverage check:**

| Spec requirement | Covered by |
|---|---|
| `socket_path()` uses `$XDG_RUNTIME_DIR` with fallback | Task 1 |
| `try_signal_existing()` writes `"show\n"`, removes stale socket | Task 2 |
| `start_listener()` spawns thread, sends on channel | Task 2 |
| `start_listener()` wakes event loop via `ctx.request_repaint()` | Task 2 (design gap filled: spec omits this but without it the window is slow to appear) |
| `main.rs` single-instance guard | Task 3 |
| `main.rs` hidden viewport at startup | Task 3 |
| `app.rs` `show_receiver` field | Task 4 |
| `app.rs` drains show signals, resets state | Task 4 |
| `ui.rs` Escape hides instead of closes | Task 5 |
| `ui.rs` Enter hides instead of closes | Task 5 |
| Stale socket edge case | Task 2 (`try_signal_existing` removes file on connect failure) |
| Multiple rapid signals | Task 4 (`while let Ok` drains all in one frame) |
| `install.sh` hotkey instructions | Task 6 |
| No new crate dependencies | Confirmed — only `std::os::unix::net` used |

**Placeholder scan:** No TBDs, all code blocks are complete, no "similar to Task N" shortcuts.

**Type consistency:** `show_tx: crossbeam_channel::Sender<()>` and `show_rx: Receiver<()>` are used consistently across Task 3 (main.rs), Task 4 (app.rs), and Task 2 (ipc.rs). `start_listener(show_tx, ctx)` signature matches the call site in Task 4.
