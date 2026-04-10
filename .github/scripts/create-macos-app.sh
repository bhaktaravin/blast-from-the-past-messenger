#!/bin/bash
set -e

APP="target/release/Blast From The Past.app"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"
cp target/release/chatmessagediscordclone "$APP/Contents/MacOS/"

# Create Info.plist
cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>chatmessagediscordclone</string>
    <key>CFBundleIdentifier</key>
    <string>com.blastfromthepast.messenger</string>
    <key>CFBundleName</key>
    <string>Blast From The Past</string>
    <key>CFBundleVersion</key>
    <string>1.2.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.2.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Install create-dmg if not already installed
if ! command -v create-dmg &> /dev/null; then
    echo "Installing create-dmg..."
    brew install create-dmg
fi

# Create DMG
create-dmg \
  --volname "Blast From The Past" \
  --window-size 600 400 \
  --icon-size 100 \
  --app-drop-link 450 200 \
  "blast-from-the-past-macos.dmg" \
  "$APP" || {
    echo "create-dmg failed, creating simple DMG..."
    hdiutil create -volname "Blast From The Past" -srcfolder "$APP" -ov -format UDZO "blast-from-the-past-macos.dmg"
  }
