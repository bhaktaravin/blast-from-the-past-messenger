# 🚀 Releases & Downloads

## Automated Builds

Every release automatically builds for:
- 🍎 **macOS** (.dmg)
- 🪟 **Windows** (.zip with .exe)
- 🐧 **Linux** (.tar.gz)
- 🌐 **Web** (GitHub Pages)

## How to Create a Release

### Option 1: Create a Git Tag
```bash
git tag -a v1.0.0 -m "Release v1.0.0"
git push origin v1.0.0
```

GitHub Actions will automatically:
1. Build for all platforms
2. Create a release
3. Upload all binaries

### Option 2: Manual Trigger
1. Go to GitHub Actions
2. Select "Release Builds"
3. Click "Run workflow"

## Download Links

Once released, binaries will be available at:
```
https://github.com/YOUR_USERNAME/blast-from-the-past-messenger/releases
```

### macOS (.dmg)
- Download: `blast-from-the-past-macos.dmg`
- Double-click to mount
- Drag app to Applications folder
- **Note**: May need to allow in System Preferences > Security

### Windows (.zip)
- Download: `blast-from-the-past-windows.zip`
- Extract anywhere
- Run `chatmessagediscordclone.exe`
- Assets folder must be in same directory

### Linux (.tar.gz)
- Download: `blast-from-the-past-linux-x86_64.tar.gz`
- Extract: `tar -xzf blast-from-the-past-linux-x86_64.tar.gz`
- Run: `./chatmessagediscordclone`
- May need to: `chmod +x chatmessagediscordclone`

### Web Version
- Automatically deployed to GitHub Pages
- Access at: `https://YOUR_USERNAME.github.io/blast-from-the-past-messenger`
- No download needed!

## Build Matrix

| Platform | Workflow | Output | Size |
|----------|----------|--------|------|
| macOS | `build-macos` | `.dmg` | ~15 MB |
| Windows | `build-windows` | `.zip` | ~10 MB |
| Linux | `build-linux` | `.tar.gz` | ~12 MB |
| Web | `web-deploy` | GitHub Pages | ~1 MB |

## Workflow Files

- `.github/workflows/release.yml` - Builds native apps
- `.github/workflows/web-deploy.yml` - Deploys web version

## Manual Building

If you prefer to build locally:

### Native
```bash
cargo build --release --bin chatmessagediscordclone
```

### Web
```bash
trunk build --release
```

## Continuous Deployment

- **Native apps**: Only on tagged releases (`v*.*.*`)
- **Web version**: Every push to `main` branch

## First Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md` (if you have one)
- [ ] Commit changes
- [ ] Create and push tag
- [ ] Wait for GitHub Actions to complete
- [ ] Verify downloads work
- [ ] Announce release! 🎉

## Troubleshooting

### macOS: "Cannot verify developer"
- Right-click app → Open → Open anyway
- Or: System Preferences → Security → Allow

### Windows: "Windows Protected"
- Click "More info" → "Run anyway"
- This happens because the app isn't signed

### Linux: Permission denied
```bash
chmod +x chatmessagediscordclone
./chatmessagediscordclone
```

## Code Signing (Optional)

For a better user experience, consider code signing:

- **macOS**: Apple Developer account ($99/year)
- **Windows**: Code signing certificate (~$70-400/year)
- **Linux**: Not required

GitHub Actions can be configured to sign if you have certificates.

## Support

If builds fail:
1. Check GitHub Actions logs
2. Verify `Cargo.toml` is correct
3. Ensure all dependencies are compatible
4. Open an issue on GitHub

---

**Ready to release?** Just push a tag! 🚀
