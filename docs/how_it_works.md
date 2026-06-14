# How Clipwise Works

This document explains the architecture, data flow, and storage layout of Clipwise.

---

## Overview

Clipwise is a single-process application with two concurrent execution paths:

- A **background thread** that polls the system clipboard every 500 ms
- The **UI thread** that renders the window, handles input, and manages state

They communicate through a lock-free channel. No daemon, no IPC socket, no external process.

```
┌──────────────────────────────────────────────────────┐
│                   clipwise process                   │
│                                                      │
│  ┌────────────────────┐  channel  ┌───────────────┐  │
│  │  Clipboard watcher │ ────────► │   UI thread   │  │
│  │      thread        │           │ (egui/eframe) │  │
│  └────────────────────┘           └───────┬───────┘  │
│                                           │          │
│                                    ┌──────▼──────┐   │
│                                    │   sled DB   │   │
│                                    │  (~/.local) │   │
│                                    └─────────────┘   │
└──────────────────────────────────────────────────────┘
```

---

## Startup Sequence

`main.rs` runs these steps before the window appears:

1. Opens the sled database at `~/.local/share/clipwise/db` (creates it on first run)
2. Calls `storage.load_all()` to restore the full history into a `Vec<ClipboardItem>`
3. Creates a `crossbeam` unbounded channel `(tx, rx)`
4. Spawns the clipboard watcher thread, handing it `tx`
5. Launches the egui window via `eframe::run_native`, handing it `rx` and the loaded items

---

## Clipboard Watcher Thread

Defined in `clipboard.rs`. Runs a simple poll loop forever:

```
loop:
  text = arboard::Clipboard::get_text()
  if text is non-empty AND text != last_seen:
    build ClipboardItem { uuid, content, timestamp, pinned: false }
    send item over channel tx
    last_seen = text
  sleep 500ms
```

The watcher only sends. It never reads from storage, never mutates shared state.

---

## UI Thread — Per-Frame Update

`eframe` calls `ClipwiseApp::update()` on every rendered frame. The update function does:

### 1. Drain the channel

Pull every pending `ClipboardItem` off the channel:

- **Duplicate found** — remove the existing entry, update its `copied_at` to now, re-insert at position 0
- **New item** — prepend to the list; if unpinned items now exceed 500, drop the oldest unpinned one

After each insertion the list is re-sorted: pinned items first (newest → oldest), then unpinned (newest → oldest). The result is written to storage.

### 2. Render the UI

`ui::render()` is called with a mutable reference to the app state. It:

1. Computes the **filtered list** — if the search query is empty, all items are shown in sort order; otherwise items are scored with `SkimMatcherV2` and only matches are shown (pinned matches first)
2. Reads all keyboard input in one `ctx.input()` call
3. Handles keyboard actions (navigation, copy-on-enter, pin toggle, delete)
4. Renders the central panel: search bar → PINNED section → RECENT section
5. Collects any button clicks from item rows as a `RowAction`
6. Applies the collected action (toggle pin, confirm delete, etc.) and saves to storage

UI mutations are never applied mid-render — they are collected during rendering and applied after the frame is built. This avoids borrow conflicts and keeps rendering logic read-only.

---

## Data Model

```rust
pub struct ClipboardItem {
    pub id: String,              // UUIDv4
    pub content: String,         // The copied text
    pub copied_at: DateTime<Utc>,
    pub pinned: bool,
}
```

In memory, items are held in a single `Vec<ClipboardItem>` on `ClipwiseApp`. Sort order is always maintained: pinned items first, each group sorted newest → oldest.

---

## Storage Layout

Clipboard history lives in an embedded [sled](https://github.com/spacejam/sled) database:

```
~/.local/share/clipwise/db/
```

Sled is a key-value store — the directory contains its internal files, not a single `.db` file. Clipwise uses one named tree inside the database called `items`.

| Key | Value |
|---|---|
| `<uuid-string>` | JSON-serialised `ClipboardItem` |
| `__order__` | JSON array of UUID strings in display order |

The `__order__` key drives `load_all()`: on startup, its array is read first, then each UUID is fetched in order to reconstruct the list. This guarantees the window opens with exactly the same ordering the user last saw.

Most mutations use targeted writes rather than a full rewrite:

- **Dedup (existing item bumped to top)** — `save_item_and_order()` upserts the single changed item and rewrites `__order__`, then flushes.
- **Delete** — `delete_item_and_order()` removes the single key and rewrites `__order__`, then flushes.
- **New item / pin toggle** — `save_all()` is still used here; it clears the tree, writes all items, and writes `__order__`, then flushes.

**To wipe all history:**
```bash
rm -rf ~/.local/share/clipwise/db
```

---

## Module Responsibilities

| File | Responsibility |
|---|---|
| `main.rs` | Wire up storage, watcher thread, and egui window |
| `clipboard.rs` | `ClipboardItem` struct; background watcher thread |
| `storage.rs` | Open sled DB; `load_all`; `save_all`; `save_item_and_order`; `delete_item_and_order` |
| `app.rs` | `ClipwiseApp` state; `eframe::App::update` loop; sort logic |
| `ui.rs` | All egui rendering; fuzzy filter; keyboard input; row actions |
| `theme.rs` | `Color32` constants; `setup_visuals` applying the dark theme |

---

## Key Design Decisions

**No daemon.** The watcher runs as a thread inside the same process. When the window closes, the watcher stops too. This keeps the install simple (single binary) at the cost of only capturing history while the app is running.

**Immediate-mode UI.** egui redraws the entire UI every frame. There is no retained widget tree to sync. All state lives in `ClipwiseApp`; the render functions are pure in spirit — they read state, emit paint commands, and return actions.

**Collect-then-apply for UI mutations.** Item rows return an `Option<RowAction>` rather than mutating app state directly inside egui closures. This avoids Rust borrow conflicts and makes the rendering path easy to follow.

**Targeted storage writes for hot paths.** The dedup path (same content copied again) and delete use `save_item_and_order` / `delete_item_and_order`, which touch only the affected sled key plus `__order__`. `save_all` (clear + full rewrite) is reserved for structural changes — new items and pin toggles — where the full order may shift.
