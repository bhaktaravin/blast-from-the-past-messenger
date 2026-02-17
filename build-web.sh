#!/bin/bash

# Build script for web version

set -e

echo "🌐 Building Blast From The Past Messenger for Web..."

# Check if trunk is installed
if ! command -v trunk &> /dev/null; then
    echo "❌ Trunk is not installed!"
    echo "📦 Installing trunk..."
    cargo install --locked trunk
fi

# Check if wasm32 target is installed
if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
    echo "📦 Installing wasm32 target..."
    rustup target add wasm32-unknown-unknown
fi

# Build for web
echo "🔨 Building..."
trunk build --release

echo "✅ Build complete!"
echo "📁 Output in: dist/"
echo ""
echo "To serve locally, run:"
echo "  ./serve-web.sh"
echo ""
echo "Or use trunk to build and serve:"
echo "  trunk serve --open"
