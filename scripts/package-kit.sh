#!/usr/bin/env bash
# Build a clean zip for Gumroad (or email): source tree without build artifacts or .git.
# Usage: ./scripts/package-kit.sh
# Optional: BFTP_OUTPUT_DIR=/path ./scripts/package-kit.sh  (default: ~/Desktop)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

STAMP="$(date +%Y%m%d-%H%M)"
OUT_NAME="blast-from-the-past-selfhost-kit-${STAMP}.zip"
OUT_DIR="${BFTP_OUTPUT_DIR:-$HOME/Desktop}"
mkdir -p "$OUT_DIR"
OUT_PATH="$OUT_DIR/$OUT_NAME"

if git rev-parse --git-dir >/dev/null 2>&1; then
  git archive --format=zip -9 --prefix=blast-from-the-past-messenger/ HEAD -o "$OUT_PATH"
else
  echo "Warning: not a git repo; zipping working tree (slower, may include cruft)." >&2
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  name="blast-from-the-past-messenger"
  mkdir -p "$tmp/$name"
  rsync -a --exclude='.git/' --exclude='target/' --exclude='dist/' --exclude='_screenshot_read.png' \
    "$ROOT/" "$tmp/$name/"
  (cd "$tmp" && zip -qr -9 "$OUT_PATH" "$name")
fi

echo "Created: $OUT_PATH"
echo "Start here after unzip: blast-from-the-past-messenger/SELF_HOST_KIT.md"
