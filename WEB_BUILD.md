# Web Version Guide

This guide explains how to build and run the web (WASM) version of Blast From The Past Messenger.

## Why Web Version?

The web version runs directly in your browser, avoiding:
- Code signing issues on macOS
- Installation hassles
- Permission dialogs
- Platform-specific builds

## Prerequisites

You need:
1. **Rust** - Already installed ✅
2. **Trunk** - Web build tool (will be auto-installed)
3. **wasm32 target** - WebAssembly target (will be auto-installed)

## Quick Start

### Option 1: Using the Scripts (Recommended)

```bash
# Build and serve (opens browser automatically)
./serve-web.sh

# Or just build (output in dist/)
./build-web.sh
```

### Option 2: Manual Commands

```bash
# Install prerequisites (one-time setup)
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Serve with hot reload (recommended for development)
trunk serve --open

# Or build for production
trunk build --release
```

## Accessing the App

Once running, open your browser to:
- **Local**: http://127.0.0.1:8080
- **Network**: http://YOUR_IP:8080 (for testing on other devices)

## Current Limitations

### ⚠️ Known Issues in Web Version

1. **Networking**: WebSocket connection uses different API than native
   - Currently shows stub implementation
   - Will be fully implemented in next update

2. **Audio**: Sound effects are disabled on web
   - Browser audio APIs differ from native
   - Planned for future implementation

3. **Performance**: Slightly slower than native
   - WASM has small overhead
   - Still very usable

### ✅ What Works

- ✅ Full UI rendering
- ✅ Buddy list with grouping and status icons
- ✅ Theme support
- ✅ All visual features
- ✅ Local state management
- ⚠️ Networking (in progress)
- ❌ Sound effects (planned)

## Development Workflow

### Live Development

```bash
trunk serve --open
```

- Watches for file changes
- Auto-rebuilds on save
- Auto-refreshes browser
- Perfect for rapid iteration

### Production Build

```bash
trunk build --release
```

Output goes to `dist/` directory with:
- `index.html` - Entry point
- `*.wasm` - Compiled WebAssembly
- `*.js` - JavaScript glue code
- Optimized and minified

### Deploy to Web

After building, upload the `dist/` folder to any static hosting:

```bash
# Examples:
# GitHub Pages
cp -r dist/* docs/

# Netlify
netlify deploy --dir=dist --prod

# Vercel
vercel deploy dist --prod

# Or use any static host (Cloudflare Pages, AWS S3, etc.)
```

## File Structure

```
blast-from-the-past-messenger/
├── index.html           # HTML template for web
├── Trunk.toml          # Trunk configuration
├── build-web.sh        # Build script
├── serve-web.sh        # Development server script
├── src/
│   ├── main.rs         # Entry point (native + web)
│   ├── audio.rs        # Audio (native only)
│   └── protocol.rs     # Shared protocol
└── dist/               # Build output (generated)
```

## Troubleshooting

### "trunk: command not found"

Install trunk:
```bash
cargo install --locked trunk
```

### "error: failed to run custom build command for openssl-sys"

This shouldn't happen with web builds (no OpenSSL needed), but if it does:
```bash
# On macOS with Homebrew:
brew install openssl
export OPENSSL_DIR=$(brew --prefix openssl)
```

### Blank page or loading forever

Check browser console (F12) for errors. Common issues:
- CORS errors → Serve from same origin
- WASM not loading → Check file paths
- JavaScript errors → Clear cache and rebuild

### "wasm32-unknown-unknown" not found

Install the target:
```bash
rustup target add wasm32-unknown-unknown
```

### Build fails with memory error

The WASM build needs more memory:
```bash
# Increase Node.js memory (if using Node-based tooling)
export NODE_OPTIONS="--max-old-space-size=4096"
```

## Browser Compatibility

**Tested and Working:**
- ✅ Chrome/Chromium 90+
- ✅ Firefox 89+
- ✅ Safari 15+
- ✅ Edge 90+

**Requirements:**
- WebAssembly support (all modern browsers have this)
- JavaScript enabled
- WebGL support (for rendering)

## Performance Tips

1. **Use Release Build**: Always use `--release` for production
   ```bash
   trunk build --release
   ```

2. **Enable Compression**: Most web servers automatically compress .wasm files

3. **Browser Caching**: Serve with proper cache headers
   ```nginx
   # Nginx example
   location ~* \.(wasm|js)$ {
       expires 1y;
       add_header Cache-Control "public, immutable";
   }
   ```

4. **Use CDN**: Serve static files from a CDN for faster loading

## Next Steps

To fully implement networking on web:
1. Replace tokio WebSocket with web_sys WebSocket
2. Use wasm_bindgen_futures for async operations
3. Handle browser CORS restrictions
4. Implement reconnection logic

Audio implementation:
1. Use Web Audio API
2. Base64 encode audio files or load from URLs
3. Handle browser autoplay policies

## Questions?

- Check the main README.md for general information
- See the plan document for implementation details
- File an issue on GitHub for bugs or feature requests

## Advantages of Web Version

✅ **No Installation**: Runs directly in browser
✅ **Cross-Platform**: Works on any OS with a modern browser
✅ **No Signing**: Bypasses macOS code signing issues
✅ **Easy Updates**: Just refresh the page
✅ **Shareable**: Send a URL to friends
✅ **Mobile Support**: Works on phones/tablets
✅ **Sandboxed**: Browser security model

Happy messaging! 💬✨
