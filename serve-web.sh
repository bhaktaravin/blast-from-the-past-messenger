#!/bin/bash

# Serve script for web version

set -e

echo "🚀 Starting Blast From The Past Messenger..."

# Check if dist exists
if [ ! -d "dist" ]; then
    echo "❌ dist/ directory not found!"
    echo "📦 Building first..."
    ./build-web.sh
fi

# Serve using trunk (with hot reload)
echo "🌐 Starting development server at http://127.0.0.1:8080"
echo "Press Ctrl+C to stop"
echo ""

trunk serve --open
