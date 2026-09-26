#!/bin/bash
# Packs the (already signed + stapled) app into $1, e.g. blast-from-the-past-macos-silicon.dmg
set -e

DMG="$1"
APP="target/release/Blast From The Past.app"

# Install create-dmg if not already installed
if ! command -v create-dmg &> /dev/null; then
    echo "Installing create-dmg..."
    brew install create-dmg
fi

# create-dmg wants a folder holding the app; ditto keeps the signature and stapled ticket
STAGE="target/dmg-src"
rm -rf "$STAGE" "$DMG"
mkdir -p "$STAGE"
ditto "$APP" "$STAGE/Blast From The Past.app"

create-dmg \
  --volname "Blast From The Past" \
  --window-size 600 400 \
  --icon-size 100 \
  --icon "Blast From The Past.app" 150 200 \
  --app-drop-link 450 200 \
  "$DMG" \
  "$STAGE" || {
    echo "create-dmg failed, creating simple DMG..."
    ln -sfn /Applications "$STAGE/Applications"
    hdiutil create -volname "Blast From The Past" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
  }
