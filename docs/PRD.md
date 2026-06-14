# PRD: Clipwise — Linux Clipboard Manager

## Overview

Clipwise is a personal, single-user Linux clipboard manager built in Rust. It monitors the system clipboard, persists all copied items across reboots, supports pinning important entries, and allows deletion of any item. The UI mimics Raycast: a dark, floating launcher panel with a search bar at the top, a clean list of results below, and keyboard-first navigation.

---

## Goals

- Capture every text item copied to the system clipboard automatically.
- Persist clipboard history across shutdowns and restarts.
- Allow the user to pin important items so they always appear at the top.
- Allow the user to delete any individual item.
- Provide a fast fuzzy-search over all items.
- Look and feel like Raycast: minimal, dark, launcher-style UI.
- Single binary, no daemon required (though a background thread handles clipboard polling).

---

## Tech Stack

| Concern | Choice | Reason |
|---|---|---|
| Language | Rust (2021 edition) | Performance, single binary, memory safety |
| GUI | `eframe` + `egui` 0.28 | Pure-Rust immediate-mode UI, easy custom styling |
| Clipboard access | `arboard` 3.x | Cross-platform clipboard read/write on Linux (X11 + Wayland) |
| Persistence | `sled` 0.34 | Embedded key-value store, no external DB needed |
| Serialization | `serde` + `serde_json` | Serialize `ClipboardItem` structs to/from sled |
| Fuzzy search | `fuzzy-matcher` 0.3 | Skim-style fuzzy scoring |
| IDs | `uuid` v4 | Stable unique key per clipboard item |
| Timestamps | `chrono` with serde feature | Human-readable "copied at" times |
| Channels | `crossbeam-channel` | Send new clipboard events from watcher thread to UI thread |

### Cargo.toml (exact)

```toml
[package]
name = "clipwise"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = { version = "0.28", features = ["default"] }
egui = "0.28"
arboard = "3.4"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sled = "0.34"
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1.8", features = ["v4"] }
fuzzy-matcher = "0.3"
crossbeam-channel = "0.5"

[profile.release]
opt-level = 3
lto = true
strip = true
```

---

## Data Model

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClipboardItem {
    pub id: String,           // UUIDv4
    pub content: String,      // The copied text
    pub copied_at: DateTime<Utc>,
    pub pinned: bool,
}
```

### Persistence Layout (sled)

- Tree name: `"items"`
- Key: item UUID (UTF-8 bytes)
- Value: `serde_json` bytes of `ClipboardItem`
- Special key `"__order__"`: JSON array of UUIDs representing display order (pinned first, then recency descending)
- DB path: `~/.local/share/clipwise/db`

On every mutation (add, delete, pin/unpin), call `save_all(&items)` which rewrites the order key and upserts all items.

On startup, call `load_all()` which reads the order key, then fetches each item by ID in that order.

---

## Module Structure

```
src/
  main.rs        — entry point: init storage, spawn clipboard watcher, launch egui app
  clipboard.rs   — ClipboardItem struct + clipboard watcher thread
  storage.rs     — sled open/save_all/load_all
  app.rs         — ClipwiseApp struct implementing eframe::App
  ui.rs          — all egui rendering logic (search bar, list, item row, actions)
  theme.rs       — color constants and egui Visuals setup
```

---

## Architecture

### Startup Sequence

1. `main()` opens `Storage`, calls `load_all()` to restore items.
2. Spawns a clipboard watcher thread (see below).
3. Launches `eframe::run_native()` with `ClipwiseApp`.

### Clipboard Watcher Thread

- Runs in a `std::thread::spawn` loop.
- Uses `arboard::Clipboard::new()` to poll `get_text()` every **500 ms**.
- Keeps track of `last_seen: String`.
- When the clipboard text changes and is non-empty and not already in history:
  - Creates a new `ClipboardItem` with a new UUIDv4.
  - Sends it over a `crossbeam_channel::Sender<ClipboardItem>` to the UI thread.
- The UI thread's `update()` drains the receiver each frame and prepends new items to the list.

### ClipwiseApp State

```rust
pub struct ClipwiseApp {
    items: Vec<ClipboardItem>,
    search_query: String,
    storage: Storage,
    receiver: Receiver<ClipboardItem>,
    confirm_delete: Option<String>,   // item ID pending delete confirmation
}
```

---

## UI Specification

### Window

- **Style**: Frameless floating window, no native title bar.
- **Size**: 680 × 520 px, not resizable.
- **Position**: Centered on screen at launch.
- **Background**: `#1C1C1E` (near-black, like Raycast).
- **Corner radius**: 12 px on the outer window frame.
- **Shadow**: Enabled via `eframe` window decorations or custom drop shadow if possible.

### Layout (top to bottom)

```
┌─────────────────────────────────────────┐
│  🔍  Search clipboard history...        │  ← Search bar (40 px tall)
├─────────────────────────────────────────┤
│  📌 PINNED                              │  ← Section header (only if pinned items exist)
│  ┌───────────────────────────────────┐  │
│  │ [pin icon]  Item text preview...  │  │  ← Pinned item row
│  └───────────────────────────────────┘  │
│  RECENT                                 │  ← Section header
│  ┌───────────────────────────────────┐  │
│  │ [pin icon]  Item text preview...  │  │  ← Regular item row (selected = highlighted)
│  └───────────────────────────────────┘  │
│  ...                                    │
└─────────────────────────────────────────┘
```

### Search Bar

- Full-width text input at the very top, always focused on launch.
- Placeholder: `"Search clipboard history…"`
- Icon: magnifying glass (`🔍`) on the left, rendered as text or egui icon.
- Background: `#2C2C2E`, text color `#F5F5F7`, placeholder color `#6E6E73`.
- No border; subtle inner shadow.
- Typing filters the list in real-time using fuzzy matching (score from `fuzzy-matcher`).
- Pressing `Escape` clears the search if non-empty; if already empty, hides/quits the window.

### Item Row

Each row is **56 px tall** and contains:

- **Left**: Pin icon (filled star `★` if pinned, outline `☆` if not). Clicking toggles pin.
- **Center**: Truncated single-line preview of content (max ~80 chars, ellipsis). Below it in smaller text: relative timestamp (`"2 minutes ago"`, `"Yesterday"`, etc.).
- **Right**: Trash icon (`🗑`) — clicking triggers delete confirmation inline.

Row states:
- **Default**: background `#1C1C1E`
- **Hovered**: background `#2C2C2E`
- **Selected** (keyboard nav): background `#3A3A3C`, left accent bar `#0A84FF` (2 px wide, Apple blue)
- **Pinned rows** always appear above unpinned rows regardless of recency.

### Section Headers

- Text: `"PINNED"` / `"RECENT"` in uppercase, `10 px`, color `#6E6E73`.
- Shown only when the relevant section is non-empty.
- No separator line — spacing alone (8 px padding) separates sections.

### Delete Flow

1. User clicks the trash icon on a row.
2. That row expands slightly and shows inline: `"Delete this item?"` with two small buttons: **`Cancel`** and **`Delete`** (red, `#FF453A`).
3. Clicking **Delete** removes the item from `items`, calls `storage.save_all()`, and collapses the row.
4. Clicking **Cancel** collapses the row back to normal.
5. Only one row can be in "confirm delete" state at a time.

### Keyboard Navigation

| Key | Action |
|---|---|
| `↑` / `↓` | Move selection up/down the filtered list |
| `Enter` | Copy selected item back to clipboard and close window |
| `Ctrl+D` | Toggle pin on selected item |
| `Delete` or `Backspace` (when search empty) | Open delete confirmation for selected item |
| `Escape` | Clear search if non-empty; else quit |

### Theme / Colors

Define all colors as constants in `theme.rs`:

```rust
pub const BG_PRIMARY:   egui::Color32 = egui::Color32::from_rgb(28,  28,  30);   // #1C1C1E
pub const BG_ELEVATED:  egui::Color32 = egui::Color32::from_rgb(44,  44,  46);   // #2C2C2E
pub const BG_SELECTED:  egui::Color32 = egui::Color32::from_rgb(58,  58,  60);   // #3A3A3C
pub const ACCENT_BLUE:  egui::Color32 = egui::Color32::from_rgb(10,  132, 255);  // #0A84FF
pub const ACCENT_RED:   egui::Color32 = egui::Color32::from_rgb(255, 69,  58);   // #FF453A
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(245, 245, 247);  // #F5F5F7
pub const TEXT_MUTED:   egui::Color32 = egui::Color32::from_rgb(110, 110, 115);  // #6E6E73
pub const ACCENT_GOLD:  egui::Color32 = egui::Color32::from_rgb(255, 214, 10);   // #FFD60A (pin icon)
```

Apply these in a `setup_visuals(ctx: &egui::Context)` function called once at startup:
- Set `Visuals::dark()` as base.
- Override `panel_fill`, `window_fill`, `extreme_bg_color`, `code_bg_color` with `BG_PRIMARY`.
- Set `selection.bg_fill` to `BG_SELECTED`.
- Set `widgets.inactive.bg_fill`, `widgets.hovered.bg_fill` appropriately.
- Set `override_text_color` to `TEXT_PRIMARY`.

---

## Behavior Details

### Deduplication

Before inserting a new item from the watcher, check if `items` already contains an entry with the same `content` (exact match). If yes, move that existing entry to the top (update `copied_at`) instead of creating a duplicate. Re-save storage.

### Pinned Item Ordering

Display order is always: all pinned items (sorted by `copied_at` descending) followed by all unpinned items (sorted by `copied_at` descending). The `__order__` key in sled stores this final sorted order.

### Max History

Cap history at **500 items** (unpinned). When the cap is exceeded, drop the oldest unpinned item. Pinned items are never auto-dropped.

### Copy-on-Select

When the user presses `Enter` on a selected item, write `item.content` back to the system clipboard via `arboard` and close the application window (or hide it).

### Timestamps

Use `chrono` to compute human-readable relative times:
- < 1 min → `"Just now"`
- < 1 hour → `"X minutes ago"`
- < 24 hours → `"X hours ago"`
- < 7 days → `"Yesterday"` / `"X days ago"`
- Older → formatted date `"Jun 3, 2025"`

---

## File: `src/main.rs`

Responsibilities:
1. Initialize `Storage::open()` — panic with a readable message on failure.
2. Call `storage.load_all()` to get initial items.
3. Create a `crossbeam_channel::unbounded()` pair `(tx, rx)`.
4. Spawn clipboard watcher thread: `clipboard::start_watcher(tx)`.
5. Build `eframe::NativeOptions` with: `initial_window_size`, `decorated: false`, `centered: true`, `always_on_top: true`.
6. Call `eframe::run_native("Clipwise", options, Box::new(|cc| Box::new(ClipwiseApp::new(cc, items, storage, rx))))`.

---

## File: `src/clipboard.rs`

Responsibilities:
1. Define `ClipboardItem` struct (with serde derives).
2. Define `start_watcher(tx: Sender<ClipboardItem>)` — spawns a thread that polls `arboard::Clipboard` every 500 ms and sends new items on `tx`.

---

## File: `src/storage.rs`

Responsibilities:
1. Define `Storage` struct wrapping `sled::Db`.
2. `Storage::open() -> Result<Self, Box<dyn Error>>` — opens DB at `~/.local/share/clipwise/db`.
3. `save_all(&self, items: &[ClipboardItem]) -> Result<()>` — clears tree, writes all items + order key.
4. `load_all(&self) -> Result<Vec<ClipboardItem>>` — reads order key, fetches items in order.

---

## File: `src/app.rs`

Responsibilities:
1. Define `ClipwiseApp` struct.
2. Implement `eframe::App` for it — `update()` should:
   a. Drain `self.receiver` and merge new items (with dedup + cap logic).
   b. Call `ui::render(ctx, self)`.

---

## File: `src/ui.rs`

Responsibilities:
1. `pub fn render(ctx: &egui::Context, app: &mut ClipwiseApp)` — main render function.
2. Renders the central panel with: search bar, section headers, scrollable item list.
3. Handles all click events (pin toggle, delete confirmation, item selection).
4. Handles keyboard shortcuts.
5. Calls `app.storage.save_all()` after any mutation.

---

## File: `src/theme.rs`

Responsibilities:
1. Define all `Color32` constants.
2. `pub fn setup_visuals(ctx: &egui::Context)` — applies the dark Raycast-like theme to egui.

---

## Install & Run Instructions (include in README.md)

```bash
# Prerequisites
sudo apt install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

# Build
cargo build --release

# Run
./target/release/clipwise

# Optional: add to autostart
cp target/release/clipwise ~/.local/bin/
```

---

## Out of Scope (v1)

- Image clipboard support (text only).
- Multi-user or cloud sync.
- Global hotkey to summon the window (user launches binary manually or binds a hotkey in their WM).
- Rich text or HTML clipboard entries.
- Tags or custom categories beyond pin.
