#!/usr/bin/env sh
# Installs RAM Altman for the current user (no root needed).
#   ./install.sh              install
#   ./install.sh --autostart  install and start automatically on login
#   ./install.sh --uninstall  remove everything this script installed
set -eu
HERE=$(cd "$(dirname "$0")" && pwd)
BIN_DIR="$HOME/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
AUTOSTART_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/autostart"

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "$BIN_DIR/ram-altman" \
          "$DATA_DIR/applications/ram-altman.desktop" \
          "$DATA_DIR/icons/hicolor/256x256/apps/ram-altman.png" \
          "$AUTOSTART_DIR/ram-altman.desktop"
    echo "RAM Altman uninstalled."
    exit 0
fi

install -Dm755 "$HERE/ram-altman" "$BIN_DIR/ram-altman"
install -Dm644 "$HERE/ram-altman.png" "$DATA_DIR/icons/hicolor/256x256/apps/ram-altman.png"
sed "s|^Exec=.*|Exec=$BIN_DIR/ram-altman|" "$HERE/ram-altman.desktop" > "$HERE/.desktop.tmp"
install -Dm644 "$HERE/.desktop.tmp" "$DATA_DIR/applications/ram-altman.desktop"
if [ "${1:-}" = "--autostart" ]; then
    install -Dm644 "$HERE/.desktop.tmp" "$AUTOSTART_DIR/ram-altman.desktop"
    echo "Autostart on login enabled."
fi
rm -f "$HERE/.desktop.tmp"
command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$DATA_DIR/applications" >/dev/null 2>&1 || true

echo "RAM Altman installed. Find it in your app menu, or run: $BIN_DIR/ram-altman"
