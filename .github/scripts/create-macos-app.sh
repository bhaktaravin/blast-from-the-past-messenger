#!/bin/bash
# Wraps target/release/chatmessagediscordclone in "Blast From The Past.app".
# VERSION (e.g. 1.3.0) comes from the release tag; falls back to Cargo.toml.
# Sign and notarize the app *before* packing it with create-macos-dmg.sh, or the
# DMG ships an unsigned copy that Gatekeeper refuses to open.
set -e

VERSION="${VERSION:-$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)}"

APP="target/release/Blast From The Past.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Resources"
cp target/release/chatmessagediscordclone "$APP/Contents/MacOS/"
cp assets/icons/icon.icns "$APP/Contents/Resources/AppIcon.icns"

cat > "$APP/Contents/Info.plist" << EOF
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
    <key>CFBundleDisplayName</key>
    <string>Blast From The Past</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleVersion</key>
    <string>${VERSION}</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.social-networking</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
