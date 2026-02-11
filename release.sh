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


# --- Homebrew Cask Automation ---
CASK_REPO="$HOME/homebrew-blast-from-the-past"
CASK_FILE="$CASK_REPO/Casks/blast-from-the-past.rb"
MAC_URL="https://github.com/bhaktaravin/blast-from-the-past-messenger/releases/download/v$VERSION/$MAC_ZIP"
SHA256=$(shasum -a 256 "dist/v$VERSION/$MAC_ZIP" | awk '{print $1}')

echo "== Updating Homebrew cask formula =="

if [ ! -d "$CASK_REPO" ]; then
    git clone https://github.com/bhaktaravin/homebrew-blast-from-the-past.git "$CASK_REPO"
fi

sed -i '' "s|version \".*\"|version \"$VERSION\"|" "$CASK_FILE"
sed -i '' "s|sha256 \".*\"|sha256 \"$SHA256\"|" "$CASK_FILE"
sed -i '' "s|url \".*\"|url \"$MAC_URL\"|" "$CASK_FILE"

cd "$CASK_REPO"
git add Casks/blast-from-the-past.rb
git commit -m "Update cask to v$VERSION"
git push
cd -
echo "== Homebrew cask updated and pushed =="