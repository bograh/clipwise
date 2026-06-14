# Global Hotkey (Super+V) Design

**Date:** 2026-06-14  
**Status:** Approved

## Problem

When the user dismisses the Clipwise window (Escape or Enter), the process exits. Re-opening requires re-running the install script. There is no way to summon the window via a hotkey.

## Goal

Clipwise runs as a persistent background daemon. Pressing Super+V (bound in the user's WM) summons the window. Dismissing hides it. The process never exits unless killed.

## Approach

Single-instance daemon with Unix socket IPC. The binary has two roles:

- **Daemon role** (first run, no socket found): starts the GUI hidden, binds a Unix socket, watches clipboard.
- **Client role** (subsequent runs, socket found): writes `"show\n"` to the socket and exits immediately.

The user's WM hotkey simply runs `~/.local/bin/clipwise` — the binary figures out which role to take.

## Architecture

### New module: `src/ipc.rs`

Three functions:

```
socket_path() -> PathBuf
  Uses $XDG_RUNTIME_DIR/clipwise.sock
  Falls back to ~/.local/share/clipwise/clipwise.sock

try_signal_existing() -> bool
  Tries UnixStream::connect(socket_path())
  On success: writes "show\n", returns true  (caller should exit)
  On refused: removes stale socket file, returns false (caller becomes daemon)

start_listener(show_tx: Sender<()>)
  Binds the socket, spawns thread
  For each incoming connection: sends () on show_tx
```

### `src/main.rs`

```
1. ipc::try_signal_existing() → true  →  std::process::exit(0)
2. ipc::start_listener(show_tx)
3. ViewportBuilder: add .with_visible(false)
4. Pass show_rx into ClipwiseApp::new
```

### `src/app.rs`

Add `show_receiver: Receiver<()>` field. In `update()`, drain before rendering:

```rust
while let Ok(()) = self.show_receiver.try_recv() {
    ctx.send_viewport_cmd(ViewportCommand::Visible(true));
    ctx.send_viewport_cmd(ViewportCommand::Focus);
    self.search_query.clear();
    self.focus_requested = false;
}
```

### `src/ui.rs`

Replace both `ViewportCommand::Close` calls with hide + reset:

```rust
ctx.send_viewport_cmd(ViewportCommand::Visible(false));
app.focus_requested = false;
```

Locations:
- Escape with empty search query
- After Enter copies item to clipboard

### `install.sh`

No change to the autostart `.desktop` entry — it already runs the binary, which will become the daemon.

Add a printed block at the end with WM hotkey setup instructions for i3/sway, GNOME, and KDE.

## Data Flow

```
Login
  → autostart runs clipwise
  → no socket exists → daemon role
  → clipboard watcher starts, window hidden, socket bound

User presses Super+V
  → WM runs ~/.local/bin/clipwise
  → socket found → client role
  → writes "show" to socket → exits
  → daemon receives signal → Visible(true) + Focus
  → window appears, search bar focused, query cleared

User selects item (Enter)
  → item written to clipboard
  → Visible(false), focus_requested = false
  → window hidden, daemon still running

User presses Escape (empty search)
  → Visible(false), focus_requested = false
  → window hidden, daemon still running
```

## Edge Cases

- **Stale socket** (daemon crashed): `try_signal_existing` gets connection refused → removes socket file → this run becomes the new daemon.
- **Multiple rapid Super+V presses**: each client write sends one `()` on the channel; `try_recv` drains all of them in one frame — window stays visible, no flicker.
- **No XDG_RUNTIME_DIR**: falls back to `~/.local/share/clipwise/clipwise.sock`.

## No New Dependencies

Uses only `std::os::unix::net::{UnixListener, UnixStream}` — already available in Rust's standard library. No new crates.
