#!/usr/bin/env bash
set -e

if [ "$(id -u)" -eq 0 ]; then
    PREFIX="${PREFIX:-/usr/local}"
    DATA_DIR="$PREFIX/share"
else
    PREFIX="${PREFIX:-$HOME/.local}"
    DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
fi

BIN_DIR="$PREFIX/bin"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/128x128/apps"

echo "Uninstalling MONORYX..."
rm -f "$BIN_DIR/monoryx"
rm -f "$APP_DIR/monoryx.desktop"
rm -f "$ICON_DIR/monoryx.png"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

echo "MONORYX has been uninstalled."
