#!/usr/bin/env bash
set -e

echo "=== MONORYX Linux Installer ==="

if [ "$(id -u)" -eq 0 ]; then
    PREFIX="${PREFIX:-/usr/local}"
    BIN_DIR="$PREFIX/bin"
    DATA_DIR="$PREFIX/share"
else
    PREFIX="${PREFIX:-$HOME/.local}"
    BIN_DIR="$PREFIX/bin"
    DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
fi

APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor/128x128/apps"

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ -f "$SCRIPT_DIR/monoryx" ]; then
    EXE_SRC="$SCRIPT_DIR/monoryx"
elif [ -f "$SCRIPT_DIR/target/release/monoryx" ]; then
    EXE_SRC="$SCRIPT_DIR/target/release/monoryx"
elif [ -f "./monoryx" ]; then
    EXE_SRC="./monoryx"
elif command -v monoryx >/dev/null 2>&1; then
    EXE_SRC="$(command -v monoryx)"
else
    echo "Error: monoryx executable not found in $SCRIPT_DIR" >&2
    exit 1
fi

echo "Installing binary to $BIN_DIR/monoryx..."
install -m 755 "$EXE_SRC" "$BIN_DIR/monoryx"

if [ -f "$SCRIPT_DIR/assets/icon.png" ]; then
    echo "Installing application icon..."
    install -m 644 "$SCRIPT_DIR/assets/icon.png" "$ICON_DIR/monoryx.png"
elif [ -f "$SCRIPT_DIR/icon.png" ]; then
    echo "Installing application icon..."
    install -m 644 "$SCRIPT_DIR/icon.png" "$ICON_DIR/monoryx.png"
fi

echo "Creating desktop entry..."
cat <<EOF > "$APP_DIR/monoryx.desktop"
[Desktop Entry]
Name=MONORYX
Comment=Minecraft, without the clutter. Lightweight native launcher.
Exec=$BIN_DIR/monoryx %u
Icon=monoryx
Terminal=false
Type=Application
Categories=Game;
StartupWMClass=monoryx
MimeType=x-scheme-handler/monoryx;
EOF

chmod 644 "$APP_DIR/monoryx.desktop"

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$APP_DIR" 2>/dev/null || true
fi

echo "=== MONORYX successfully installed! ==="
echo "You can launch it from your desktop applications or run 'monoryx'."
