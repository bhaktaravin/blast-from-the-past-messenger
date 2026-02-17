# 🚀 Quick Start: Web Version

Run the messenger in your browser in 3 easy steps!

## Step 1: Install Prerequisites (One-time)

```bash
cd ~/Code/blast-from-the-past-messenger

# Install trunk (web build tool)
cargo install --locked trunk

# Install wasm32 target
rustup target add wasm32-unknown-unknown
```

This takes ~5 minutes on first run. ☕

## Step 2: Run the Development Server

```bash
./serve-web.sh
```

OR manually:

```bash
trunk serve --open
```

This will:
- ✅ Build the WASM binary
- ✅ Start a local server
- ✅ Open your browser automatically
- ✅ Watch for changes and auto-reload

## Step 3: Use the App

The app will open at **http://127.0.0.1:8080**

### Try it out:
1. **Sign In** - Use the default server or your own
2. **View UI** - All visual features work perfectly
3. **Test Buddy List** - Status icons, grouping, counts
4. **Change Themes** - Toggle between Light/Dark/Midnight Amber

## Current Status

### ✅ Working on Web:
- Full UI rendering with egui
- Buddy list with status icons (🟢 Online / 🟡 Away)
- Theme support
- Settings and controls
- All visual features

### 🚧 In Progress:
- **WebSocket connection** - Currently stubbed
  - Native uses tokio-tungstenite
  - Web needs web_sys WebSocket API
  - Coming soon!

### ❌ Not Yet Available:
- **Sound effects** - Disabled on web
  - Native uses rodio
  - Web needs Web Audio API
  - Planned for future

## Why Web Version?

### Solves Your Problem:
- ❌ **No more macOS signing issues**
- ❌ **No Homebrew password prompts**
- ❌ **No code signing certificates**
- ❌ **No Gatekeeper blocking**

### Additional Benefits:
- ✅ Run in any modern browser
- ✅ No installation needed
- ✅ Cross-platform (Windows, Mac, Linux, mobile)
- ✅ Easy to share (just send a URL)
- ✅ Auto-updates (just refresh)
- ✅ Sandboxed and secure

## Quick Commands

```bash
# Development server (with hot reload)
trunk serve --open

# Production build
trunk build --release

# Check build output
ls -lh dist/

# Serve production build
python3 -m http.server 8080 --directory dist
```

## Troubleshooting

### Build takes forever?

First build downloads dependencies. Subsequent builds are fast (< 10 seconds).

### Browser shows blank page?

1. Check browser console (F12)
2. Try a hard refresh (Cmd+Shift+R on Mac)
3. Rebuild: `trunk clean && trunk serve --open`

### "trunk: command not found"?

Make sure trunk installed correctly:
```bash
cargo install --locked trunk
# Check it's in your PATH
which trunk
```

### WebSocket not connecting?

This is expected - web networking is in progress! The UI will work fine, but server communication is stubbed for now.

## File Structure

```
Your browser loads:
  index.html (HTML shell)
    ↓
  *.wasm (Your Rust code compiled to WebAssembly)
    ↓
  *.js (JavaScript glue code)
    ↓
  App runs! 🎉
```

## Next Steps

1. **Try the UI** - Everything visual works perfectly
2. **Explore Features** - Buddy list, themes, settings
3. **Wait for Networking** - Coming in next update
4. **Test on Mobile** - Works on phones/tablets too!

## Production Deployment

When ready to deploy:

```bash
# Build for production
trunk build --release

# Upload dist/ folder to any static host:
# - GitHub Pages
# - Netlify
# - Vercel
# - Cloudflare Pages
# - AWS S3
# - Any web server
```

## Support

- 📖 Full guide: See [WEB_BUILD.md](WEB_BUILD.md)
- 🐛 Issues: Check browser console for errors
- 💡 Questions: File an issue on GitHub

---

**You're all set!** 🎊

Run `./serve-web.sh` and enjoy the messenger without any macOS hassles!
