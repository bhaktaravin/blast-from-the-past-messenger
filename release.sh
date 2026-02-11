#!/bin/bash

set -e

VERSION="0.1.6"
RELEASE_DIR="dist/v$VERSION"
MAC_APP_NAME="BlastFromThePast.app"
WIN_ZIP="aol-style-messenger-windows.zip"
MAC_ZIP="BlastFromThePast-macos.zip"

echo "== Cleaning previous builds =="
cargo clean

echo "== Building release binaries =="
cargo build --release --bins

echo "== Preparing release directory =="
mkdir -p "$RELEASE_DIR"

echo "== Copying Windows binaries =="
cp target/release/server.exe "$RELEASE_DIR/"
cp target/release/chatmessagediscordclone.exe "$RELEASE_DIR/"
cp scripts/release/run.bat "$RELEASE_DIR/"

echo "== Zipping Windows release =="
cd "$RELEASE_DIR"
zip "$WIN_ZIP" server.exe chatmessagediscordclone.exe run.bat
cd ../..

echo "== Building Windows installer (requires Inno Setup/iscc) =="
if command -v iscc >/dev/null 2>&1; then
    iscc scripts/installer/installer.iss
    mv dist/blast-from-the-past-messenger-setup.exe "$RELEASE_DIR/"
else
    echo "Inno Setup (iscc) not found, skipping installer build."
fi

echo "== Copying macOS app bundle =="
cp -r target/release/$MAC_APP_NAME "$RELEASE_DIR/"

echo "== Zipping macOS app =="
cd "$RELEASE_DIR"
zip -r "$MAC_ZIP" "$MAC_APP_NAME"
cd ../..

echo "== Tagging release in git =="
git add .
git commit -m "Release v$VERSION"
git tag "v$VERSION"
git push && git push --tags

echo "== Release artifacts ready in $RELEASE_DIR =="
echo "Upload $WIN_ZIP, $MAC_ZIP, and installer to GitHub Releases for v$VERSION."