#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-1.5.0}"
BIN_DIR="${2:-target/release}"
OUTPUT_DIR="${3:-dist}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

mkdir -p "$OUTPUT_DIR"
WORK_DIR=$(mktemp -d)
trap 'rm -rf "$WORK_DIR"' EXIT

APP_BUNDLE="$WORK_DIR/MONORYX.app"
CONTENTS_DIR="$APP_BUNDLE/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

mkdir -p "$MACOS_DIR" "$RESOURCES_DIR"

if [ -f "$BIN_DIR/monoryx" ]; then
    cp "$BIN_DIR/monoryx" "$MACOS_DIR/monoryx"
elif [ -f "$ROOT_DIR/target/release/monoryx" ]; then
    cp "$ROOT_DIR/target/release/monoryx" "$MACOS_DIR/monoryx"
else
    echo "Error: monoryx binary not found in $BIN_DIR" >&2
    exit 1
fi
chmod +x "$MACOS_DIR/monoryx"

if [ -f "$BIN_DIR/monoryx-updater" ]; then
    cp "$BIN_DIR/monoryx-updater" "$MACOS_DIR/monoryx-updater"
    chmod +x "$MACOS_DIR/monoryx-updater"
elif [ -f "$ROOT_DIR/target/release/monoryx-updater" ]; then
    cp "$ROOT_DIR/target/release/monoryx-updater" "$MACOS_DIR/monoryx-updater"
    chmod +x "$MACOS_DIR/monoryx-updater"
fi

sed -e "s/1.4.0/$VERSION/g" "$SCRIPT_DIR/Info.plist" > "$CONTENTS_DIR/Info.plist"

ICON_SRC="$ROOT_DIR/assets/icon.png"
if [ -f "$ICON_SRC" ] && command -v sips >/dev/null 2>&1 && command -v iconutil >/dev/null 2>&1; then
    ICONSET_DIR="$WORK_DIR/monoryx.iconset"
    mkdir -p "$ICONSET_DIR"
    sips -z 16 16 "$ICON_SRC" --out "$ICONSET_DIR/icon_16x16.png" >/dev/null 2>&1 || true
    sips -z 32 32 "$ICON_SRC" --out "$ICONSET_DIR/icon_16x16@2x.png" >/dev/null 2>&1 || true
    sips -z 32 32 "$ICON_SRC" --out "$ICONSET_DIR/icon_32x32.png" >/dev/null 2>&1 || true
    sips -z 64 64 "$ICON_SRC" --out "$ICONSET_DIR/icon_32x32@2x.png" >/dev/null 2>&1 || true
    sips -z 128 128 "$ICON_SRC" --out "$ICONSET_DIR/icon_128x128.png" >/dev/null 2>&1 || true
    sips -z 256 256 "$ICON_SRC" --out "$ICONSET_DIR/icon_128x128@2x.png" >/dev/null 2>&1 || true
    sips -z 256 256 "$ICON_SRC" --out "$ICONSET_DIR/icon_256x256.png" >/dev/null 2>&1 || true
    sips -z 512 512 "$ICON_SRC" --out "$ICONSET_DIR/icon_256x256@2x.png" >/dev/null 2>&1 || true
    sips -z 512 512 "$ICON_SRC" --out "$ICONSET_DIR/icon_512x512.png" >/dev/null 2>&1 || true
    sips -z 1024 1024 "$ICON_SRC" --out "$ICONSET_DIR/icon_512x512@2x.png" >/dev/null 2>&1 || true
    iconutil -c icns "$ICONSET_DIR" -o "$RESOURCES_DIR/monoryx.icns" >/dev/null 2>&1 || true
fi

DMG_SRC="$WORK_DIR/dmg_root"
mkdir -p "$DMG_SRC"
cp -R "$APP_BUNDLE" "$DMG_SRC/"
ln -s /Applications "$DMG_SRC/Applications"
cp "$ROOT_DIR/README.md" "$ROOT_DIR/LICENSE" "$DMG_SRC/" 2>/dev/null || true

DMG_NAME="monoryx-v${VERSION}-macos-universal.dmg"
if command -v hdiutil >/dev/null 2>&1; then
    hdiutil create -volname "MONORYX" -srcfolder "$DMG_SRC" -ov -format UDZO "$OUTPUT_DIR/$DMG_NAME"
fi

tar -czf "$OUTPUT_DIR/monoryx-v${VERSION}-macos.tar.gz" -C "$WORK_DIR" "MONORYX.app"

echo "Packaging complete: $OUTPUT_DIR/$DMG_NAME"
