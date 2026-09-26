#!/bin/sh
# Installs Blast From The Past for the current user (no root needed), so it shows up
# in the app menu. Works on any distro, including immutable ones like Fedora Kinoite.
#   ./install.sh            install / update
#   ./install.sh --uninstall
set -e
cd "$(dirname "$0")"

BIN_DIR="$HOME/.local/bin"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}"
APP_DIR="$DATA_DIR/applications"
ICON_DIR="$DATA_DIR/icons/hicolor"

if [ "$1" = "--uninstall" ]; then
    rm -f "$BIN_DIR/blast-from-the-past" \
          "$APP_DIR/blast-from-the-past.desktop" \
          "$ICON_DIR/256x256/apps/blast-from-the-past.png" \
          "$ICON_DIR/scalable/apps/blast-from-the-past.svg"
    echo "Blast From The Past has been uninstalled."
    exit 0
fi

mkdir -p "$BIN_DIR" "$APP_DIR" "$ICON_DIR/256x256/apps" "$ICON_DIR/scalable/apps"
install -m 755 blast-from-the-past "$BIN_DIR/blast-from-the-past"
install -m 644 blast-from-the-past.png "$ICON_DIR/256x256/apps/blast-from-the-past.png"
install -m 644 blast-from-the-past.svg "$ICON_DIR/scalable/apps/blast-from-the-past.svg"
# Point the menu entry at the absolute path, since ~/.local/bin isn't always on PATH
sed "s|^Exec=.*|Exec=$BIN_DIR/blast-from-the-past|" blast-from-the-past.desktop > "$APP_DIR/blast-from-the-past.desktop"
chmod 644 "$APP_DIR/blast-from-the-past.desktop"

command -v update-desktop-database >/dev/null 2>&1 && update-desktop-database "$APP_DIR" 2>/dev/null || true

echo "Installed! Find \"Blast From The Past\" in your app menu, or run: $BIN_DIR/blast-from-the-past"
