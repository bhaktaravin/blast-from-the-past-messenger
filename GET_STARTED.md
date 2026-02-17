# 🎉 Get Started with Blast From The Past Messenger

## Choose Your Version

### 🌐 Web Version (Recommended - No macOS Issues!)

**Bypasses all macOS code signing problems!**

```bash
cd ~/Code/blast-from-the-past-messenger

# One-time setup (if not already done)
cargo install --locked trunk
rustup target add wasm32-unknown-unknown

# Run it!
./serve-web.sh
# OR
trunk serve --open
```

Opens in your browser at http://127.0.0.1:8080

**✅ Pros:**
- No code signing issues
- No Homebrew password problems
- Works on any device with a browser
- Easy to share with friends
- Zero installation

**⚠️ Current limitations:**
- Networking stubbed (coming soon)
- No sound effects yet (planned)
- Everything else works perfectly!

**📖 Read more:** [QUICKSTART_WEB.md](QUICKSTART_WEB.md)

---

### 💻 Native Version (Full Features)

**For when you want all features including audio:**

```bash
cd ~/Code/blast-from-the-past-messenger

# Build and run
cargo run --release
```

**✅ Pros:**
- All features work (audio, networking)
- Best performance
- Native OS integration

**⚠️ Cons:**
- macOS code signing required
- Platform-specific builds
- Installation needed

---

## What's New?

### Phase 1: Sound Effects ✅
- 🔔 Buddy sign-on/off sounds
- 💬 Message received/sent sounds
- 🎚️ Volume control and settings
- 🧪 Test sound buttons

### Phase 2: Buddy List Enhancements ✅
- 🟢 Online status with green icon
- 🟡 Away status with yellow icon
- 📊 Buddy counts in top bar
- 📋 Grouped by status
- 💬 Away messages in italics

### Phase 3: Web Version ✅ (NEW!)
- 🌐 Runs in browser
- 🚫 No code signing needed
- 🎨 Full UI support
- 🔄 Hot reload for development

---

## File Guide

### For Users
- **GET_STARTED.md** ← You are here
- **QUICKSTART_WEB.md** - 3-step web guide
- **WEB_BUILD.md** - Complete web reference
- **README.md** - Main project info

### For Developers
- **WEB_IMPLEMENTATION_SUMMARY.md** - Technical details
- **Implementation Plan** - Original plan document

### Scripts
- **build-web.sh** - Build for web
- **serve-web.sh** - Dev server with hot reload

---

## Quick Commands Reference

### Web Version
```bash
# Development (hot reload)
trunk serve --open

# Production build
trunk build --release

# Serve production build
python3 -m http.server 8080 --directory dist
```

### Native Version
```bash
# Development
cargo run

# Release build
cargo build --release

# Run release
./target/release/chatmessagediscordclone
```

---

## Troubleshooting

### macOS Code Signing Issues?
👉 **Use the web version!** That's exactly why we built it.

### Web version not loading?
1. Check browser console (F12)
2. Try: `trunk clean && trunk serve --open`
3. Make sure wasm32 target installed: `rustup target add wasm32-unknown-unknown`

### Build errors?
1. Update Rust: `rustup update`
2. Clean and rebuild: `cargo clean && cargo build`
3. For web: `trunk clean && trunk build`

### Performance issues?
- Use `--release` flag for optimized builds
- Web: `trunk build --release`
- Native: `cargo build --release`

---

## System Requirements

### Web Version
- Modern browser (Chrome, Firefox, Safari, Edge)
- JavaScript enabled
- WebGL support (for rendering)

### Native Version
- Rust 1.75+
- macOS 10.15+ / Linux / Windows 10+
- Audio device (for sound effects)

---

## What's Working?

| Feature | Native | Web |
|---------|--------|-----|
| UI & Themes | ✅ | ✅ |
| Buddy List | ✅ | ✅ |
| Status Icons | ✅ | ✅ |
| Grouping | ✅ | ✅ |
| Counts | ✅ | ✅ |
| Settings UI | ✅ | ✅ |
| Sound Effects | ✅ | 🚧 |
| Networking | ✅ | 🚧 |

✅ = Fully working
🚧 = In progress

---

## Need Help?

1. Check the relevant guide:
   - Web issues → [WEB_BUILD.md](WEB_BUILD.md)
   - Native issues → [README.md](README.md)

2. File an issue on GitHub with:
   - Platform (Native/Web)
   - OS and browser (if web)
   - Error messages
   - Steps to reproduce

---

## 🎊 You're All Set!

**Recommended**: Start with the web version to avoid any macOS hassles:

```bash
./serve-web.sh
```

Enjoy the messenger! 💬✨

---

*Built with 💛 using Rust, egui, and WebAssembly*
