# Clipwise

A keyboard-first clipboard manager for Linux, built in Rust. Captures everything you copy, persists it across reboots, and surfaces it through a Raycast-style launcher.

![Rust](https://img.shields.io/badge/Rust-2021-orange) ![Linux](https://img.shields.io/badge/Linux-X11%20%2F%20Wayland-blue) ![License](https://img.shields.io/badge/license-MIT-green)

---

## Features

- **Persistent history** — clipboard items survive shutdowns and restarts
- **Fuzzy search** — find any item instantly by typing
- **Pin items** — keep important entries pinned to the top permanently
- **Delete items** — remove anything with an inline confirmation prompt
- **Keyboard-first** — navigate, copy, pin, and delete without touching the mouse
- **Auto-dedup** — copying the same text twice moves it to the top instead of duplicating it
- **500-item cap** — oldest unpinned items are dropped automatically; pinned items are never removed
- **Single binary** — no daemon, no background service, no dependencies at runtime

---

## Download (no Rust required)

Grab the latest release from the [Releases page](https://github.com/bograh/clipwise/releases):

**AppImage — recommended, runs on any Linux distro:**
```bash
chmod +x Clipwise-*.AppImage
./Clipwise-*.AppImage
```

**Raw binary — requires libxcb + libwayland on the system:**
```bash
chmod +x clipwise-linux-x86_64
mv clipwise-linux-x86_64 ~/.local/bin/clipwise
clipwise
```

---

## Build from Source

Clone the repo and run the install script. It handles everything — prerequisites, Rust (if not installed), the build, adding the binary to `~/.local/bin`, and wiring up autostart:

```bash
git clone https://github.com/bograh/clipwise
cd clipwise
bash install.sh
```

The script supports **apt** (Ubuntu/Debian), **dnf** (Fedora), and **pacman** (Arch). On first run it may prompt for your sudo password to install system libraries.

---

## Manual Installation

### 1. Prerequisites

**Ubuntu / Debian**
```bash
sudo apt install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev \
                 libwayland-dev pkg-config build-essential
```

**Fedora**
```bash
sudo dnf install libxcb-devel wayland-devel pkg-config gcc
```

**Arch**
```bash
sudo pacman -S libxcb wayland pkgconf base-devel
```

### 2. Build

```bash
cargo build --release
```

### 3. Install binary

```bash
cp target/release/clipwise ~/.local/bin/
```

### 4. Autostart (optional)

To launch Clipwise automatically when you log in, create a desktop entry:

```bash
mkdir -p ~/.config/autostart
cat > ~/.config/autostart/clipwise.desktop <<EOF
[Desktop Entry]
Type=Application
Name=Clipwise
Exec=$HOME/.local/bin/clipwise
Hidden=false
X-GNOME-Autostart-enabled=true
EOF
```

---

## Usage

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
| `Delete` / `Backspace` | Open delete confirmation (when search is empty) |
| `Escape` | Clear search if non-empty, otherwise close |

---

## Data

All clipboard history is stored locally in an embedded database:

```
~/.local/share/clipwise/db
```

No data ever leaves your machine. To wipe history, delete that directory.

---

## Tech Stack

| Concern | Library |
|---|---|
| UI | `eframe` + `egui` 0.28 |
| Clipboard | `arboard` 3.x |
| Storage | `sled` 0.34 (embedded key-value) |
| Fuzzy search | `fuzzy-matcher` 0.3 |
| Serialization | `serde` + `serde_json` |
| IDs | `uuid` v4 |
| Timestamps | `chrono` |
| Channels | `crossbeam-channel` |
