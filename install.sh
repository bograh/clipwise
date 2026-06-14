#!/usr/bin/env bash
set -e

BOLD='\033[1m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()    { echo -e "${BOLD}[clipwise]${NC} $1"; }
success() { echo -e "${GREEN}[clipwise]${NC} $1"; }
warn()    { echo -e "${YELLOW}[clipwise]${NC} $1"; }
die()     { echo -e "${RED}[clipwise] ERROR:${NC} $1"; exit 1; }

# Run from the project root regardless of where the script is called from
cd "$(dirname "$0")"

# ── 1. Prerequisites ──────────────────────────────────────────────────────────

PKGS=(libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
      libwayland-dev pkg-config build-essential)

if command -v apt-get &>/dev/null; then
    MISSING=()
    for pkg in "${PKGS[@]}"; do
        dpkg -l "$pkg" 2>/dev/null | grep -q '^ii' || MISSING+=("$pkg")
    done
    if [ ${#MISSING[@]} -gt 0 ]; then
        info "Installing missing system packages: ${MISSING[*]}"
        sudo apt-get install -y "${MISSING[@]}"
    else
        info "System packages already installed."
    fi
elif command -v dnf &>/dev/null; then
    info "Fedora/RHEL detected. Installing dependencies..."
    sudo dnf install -y libxcb-devel wayland-devel pkg-config gcc
elif command -v pacman &>/dev/null; then
    info "Arch detected. Installing dependencies..."
    sudo pacman -S --needed --noconfirm libxcb wayland pkgconf base-devel
else
    warn "Unknown package manager. Please install the following manually:"
    warn "  libxcb-dev, libwayland-dev, pkg-config, build-essential"
fi

# ── 2. Rust ───────────────────────────────────────────────────────────────────

if ! command -v cargo &>/dev/null; then
    info "Rust not found. Installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

CARGO=$(command -v cargo || echo "$HOME/.cargo/bin/cargo")
"$CARGO" --version || die "cargo not found after install. Re-open your terminal and re-run this script."

# ── 3. Build ──────────────────────────────────────────────────────────────────

info "Building release binary (this may take a minute on first run)..."
"$CARGO" build --release
success "Build complete."

# ── 4. Install binary ─────────────────────────────────────────────────────────

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp target/release/clipwise "$INSTALL_DIR/clipwise"
chmod +x "$INSTALL_DIR/clipwise"
success "Installed to $INSTALL_DIR/clipwise"

if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    warn "$INSTALL_DIR is not in your PATH."
    warn "Add this line to your ~/.bashrc or ~/.zshrc:"
    warn "  export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# ── 5. Autostart ─────────────────────────────────────────────────────────────

AUTOSTART_DIR="$HOME/.config/autostart"
DESKTOP_FILE="$AUTOSTART_DIR/clipwise.desktop"
mkdir -p "$AUTOSTART_DIR"

cat > "$DESKTOP_FILE" <<EOF
[Desktop Entry]
Type=Application
Name=Clipwise
Comment=Clipboard history manager
Exec=env WINIT_UNIX_BACKEND=x11 $INSTALL_DIR/clipwise
Icon=edit-paste
Hidden=false
NoDisplay=false
StartupNotify=false
X-GNOME-Autostart-enabled=true
EOF

success "Autostart entry created at $DESKTOP_FILE"

# ── 6. Run ────────────────────────────────────────────────────────────────────

# Kill any running instance so we start fresh with the new binary.
pkill -x clipwise 2>/dev/null || true
for i in $(seq 1 10); do
    pgrep -x clipwise > /dev/null || break
    sleep 0.1
done
rm -f "${XDG_RUNTIME_DIR:-$HOME/.local/share/clipwise}/clipwise.sock"

info "Starting Clipwise daemon in background..."
"$INSTALL_DIR/clipwise" &
disown
success "Clipwise is running (window hidden)."

# ── 7. Hotkey instructions ────────────────────────────────────────────────────

echo ""
success "Bind Super+V in your window manager to: $INSTALL_DIR/clipwise"
echo ""
info "  i3 / i3-gaps — add to ~/.config/i3/config:"
info "    bindsym \$mod+v exec --no-startup-id env WINIT_UNIX_BACKEND=x11 $INSTALL_DIR/clipwise"
echo ""
info "  Sway — add to ~/.config/sway/config:"
info "    bindsym \$mod+v exec env WINIT_UNIX_BACKEND=x11 $INSTALL_DIR/clipwise"
echo ""
info "  GNOME — Settings → Keyboard → Custom Shortcuts:"
info "    Command: env WINIT_UNIX_BACKEND=x11 $INSTALL_DIR/clipwise    Shortcut: Super+V"
echo ""
info "  KDE — System Settings → Shortcuts → Custom Shortcuts:"
info "    Trigger: Super+V    Action: env WINIT_UNIX_BACKEND=x11 $INSTALL_DIR/clipwise"
echo ""
info "The autostart entry at $DESKTOP_FILE will start the daemon at login."
