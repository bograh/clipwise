# Clipwise

A keyboard-first clipboard manager for Linux, built in Rust. Captures everything you copy, persists it across reboots, and surfaces it through a Raycast-style launcher.

![Rust](https://img.shields.io/badge/Rust-2021-orange) ![Linux](https://img.shields.io/badge/Linux-X11%20%2F%20Wayland-blue)

---

## Features

- **Persistent history** — clipboard items survive shutdowns and restarts
- **Fuzzy search** — find any item instantly by typing
- **Pin items** — keep important entries pinned to the top
- **Delete items** — remove anything from your history with a confirmation prompt
- **Keyboard-first** — navigate, copy, pin, and delete without touching the mouse
- **500-item history** — oldest unpinned items are dropped automatically; pinned items are never removed

---

## Installation

### Prerequisites

```bash
sudo apt install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```

### Build from source

```bash
git clone https://github.com/yourname/clipwise
cd clipwise
cargo build --release
```

### (Optional) Add to PATH

```bash
cp target/release/clipwise ~/.local/bin/
```

---

## Usage

Launch Clipwise:

```bash
clipwise
```

Bind it to a keyboard shortcut in your window manager (e.g. `Super+V`) to summon it on demand.

### Keyboard Shortcuts

| Key | Action |
|---|---|
| `↑` / `↓` | Navigate the list |
| `Enter` | Copy selected item to clipboard and close |
| `Ctrl+D` | Toggle pin on selected item |
| `Delete` / `Backspace` | Delete selected item (when search is empty) |
| `Escape` | Clear search / close window |

---

## Data

Clipboard history is stored locally at:

```
~/.local/share/clipwise/db
```

No data ever leaves your machine.
