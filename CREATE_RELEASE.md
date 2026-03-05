# 🚀 Creating Releases with Installers

## How It Works

**Release builds (DMG, Windows, Linux) ONLY trigger on version tags!**

```
Tag format: v1.0.0, v1.0.1, v2.0.0, etc.
NOT: main branch pushes
```

---

## ✅ Check Current Release (v1.0.0)

**Go to Actions:**
https://github.com/bhaktaravin/blast-from-the-past-messenger/actions

**Look for:** "Release Builds" workflow with tag `v1.0.0`

**If you see it:**
- 🟡 Yellow = Still building (wait)
- ✅ Green = Done! Check releases page
- ❌ Red = Failed (see below)

**If you DON'T see it:**
The tag push might not have triggered it. Manual trigger below.

---

## 🎯 Option 1: Manual Trigger (Easiest)

1. Go to: https://github.com/bhaktaravin/blast-from-the-past-messenger/actions/workflows/release.yml

2. Click "Run workflow" button (top right)

3. It will ask for inputs - just click "Run workflow" again

4. Wait 10-15 minutes

5. Check releases: https://github.com/bhaktaravin/blast-from-the-past-messenger/releases

---

## 🎯 Option 2: Create New Release Tag

### For v1.0.1 (patch release):

```bash
cd ~/Code/blast-from-the-past-messenger

# Create and push new tag
git tag -a v1.0.1 -m "Release v1.0.1"
git push origin v1.0.1
```

### For v1.1.0 (minor release):

```bash
git tag -a v1.1.0 -m "Release v1.1.0"
git push origin v1.1.0
```

### For v2.0.0 (major release):

```bash
git tag -a v2.0.0 -m "Release v2.0.0"
git push origin v2.0.0
```

**This automatically triggers the Release Builds workflow!**

---

## 🎯 Option 3: Re-push v1.0.0 Tag

If v1.0.0 didn't trigger properly:

```bash
cd ~/Code/blast-from-the-past-messenger

# Delete old tag
git tag -d v1.0.0
git push origin :refs/tags/v1.0.0

# Create fresh tag
git tag -a v1.0.0 -m "Release v1.0.0 - Full feature messenger"
git push origin v1.0.0
```

---

## 📦 What Gets Built

When the workflow completes, you'll get:

### Downloads at: `/releases/tag/v1.0.0`

1. **`blast-from-the-past-macos.dmg`**
   - macOS installer
   - Double-click to install
   - No Homebrew needed

2. **`blast-from-the-past-windows.zip`**
   - Windows portable app
   - Extract and run
   - Includes .exe and assets

3. **`blast-from-the-past-linux-x86_64.tar.gz`**
   - Linux binary
   - Extract and run

---

## 🔍 Troubleshooting

### Build Failed?

Check the workflow logs:
1. Go to Actions
2. Click the failed workflow
3. Click on the failed job (macOS, Windows, or Linux)
4. Read the error logs
5. Let me know the error

### Common Issues:

**macOS build fails:**
- Usually timeout or dependency issues
- Solution: Re-run the workflow

**Windows build fails:**
- Usually cargo build issues
- Solution: Check Cargo.lock is committed

**Linux build fails:**
- Usually missing system dependencies
- Solution: Already fixed in workflow

---

## 🎬 Quick Commands

### Create patch release (v1.0.1):
```bash
cd ~/Code/blast-from-the-past-messenger
git tag -a v1.0.1 -m "Patch release"
git push origin v1.0.1
```

### Check build status:
```bash
# Open in browser
open https://github.com/bhaktaravin/blast-from-the-past-messenger/actions
```

### Check releases:
```bash
# Open in browser
open https://github.com/bhaktaravin/blast-from-the-past-messenger/releases
```

---

## 📋 Release Checklist

Before creating a new release:

- [ ] All code committed and pushed
- [ ] Tests passing (if you have them)
- [ ] Version number decided (v1.0.1, v1.1.0, etc.)
- [ ] Changelog updated (optional)
- [ ] Ready to share with users

Then:
- [ ] Create and push tag
- [ ] Wait for builds (~10-15 min)
- [ ] Verify downloads work
- [ ] Share with users! 🎉

---

## 🚀 Summary

**For installers/DMG:**
- Tag with version: `git tag -a v1.0.0 -m "Release"`
- Push tag: `git push origin v1.0.0`
- Wait for builds
- Download from releases page

**For web version:**
- Just push to main
- Auto-deploys to GitHub Pages
- No tag needed

---

**Need to trigger v1.0.0 builds right now?**

Go here and click "Run workflow":
https://github.com/bhaktaravin/blast-from-the-past-messenger/actions/workflows/release.yml
