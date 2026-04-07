# Building Blast From The Past on macOS

## Prerequisites

1. Install Rust (if not already installed):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

2. Install Xcode Command Line Tools (if not already installed):
```bash
xcode-select --install
```

## Build Steps

1. Clone the repository (if you haven't already):
```bash
git clone https://github.com/bhaktaravin/blast-from-the-past-messenger.git
cd blast-from-the-past-messenger
```

2. Build the release binary:
```bash
cargo build --release --bin chatmessagediscordclone --features client
```

3. The executable will be at:
```
target/release/chatmessagediscordclone
```

## Running the App

### Option 1: Run directly
```bash
./target/release/chatmessagediscordclone
```

### Option 2: Create an app bundle
```bash
# Create app structure
mkdir -p "Blast From The Past.app/Contents/MacOS"

# Copy binary
cp target/release/chatmessagediscordclone "Blast From The Past.app/Contents/MacOS/"

# Create Info.plist
cat > "Blast From The Past.app/Contents/Info.plist" << 'EOF'
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
    <string>1.0.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF

# Move to Applications
mv "Blast From The Past.app" /Applications/

# Now you can launch it from Spotlight or Applications folder!
```

### Option 3: Use the run script
```bash
./run.sh
```

## Quick Development Run

For development/testing:
```bash
cargo run --release --bin chatmessagediscordclone --features client
```

## Server URL

The app connects to: `wss://blast-from-the-past-messenger-production.up.railway.app`

## Troubleshooting

If you get audio errors, make sure you have the audio system libraries:
```bash
# Usually pre-installed on macOS, but if needed:
brew install portaudio
```
