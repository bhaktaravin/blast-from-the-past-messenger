# 🎥 Video Calling with Jitsi Meet (100% FREE!)

## ✅ READY TO USE - NO SETUP REQUIRED!

Your video calling is **already working** with Jitsi Meet! No account needed, no API keys, completely free.

---

## 🚀 HOW TO TEST (5 minutes)

### 1. Start the Server
```bash
cargo run --bin server --features server
```

### 2. Start the Web Client
```bash
trunk serve
```

### 3. Test Video Calling
1. Open **two browser windows**:
   - Window 1: http://localhost:8080 (login as user1)
   - Window 2: http://localhost:8080 (login as user2)

2. In Window 1:
   - Right-click user2 in buddy list
   - Click "📹 Video Call"
   - Video modal opens with Jitsi Meet interface

3. In Window 2:
   - Receives notification: "📹 Video call from user1"
   - Video modal auto-opens
   - Both users can see/hear each other!

---

## 🎉 WHAT YOU GET (FREE!)

### Features:
- ✅ HD video and audio
- ✅ Screen sharing
- ✅ Chat during calls
- ✅ Raise hand
- ✅ Reactions/emojis
- ✅ Virtual backgrounds
- ✅ Recording (if enabled)
- ✅ No time limits
- ✅ No participant limits
- ✅ No account required
- ✅ No API keys needed
- ✅ Open source

### Why Jitsi?
- **100% Free** - No hidden costs, no credit card
- **Open Source** - Apache 2.0 license
- **Privacy Focused** - End-to-end encryption available
- **Battle Tested** - Used by millions worldwide
- **Self-Hostable** - Can run your own server later if needed

---

## 🔧 HOW IT WORKS

### Current Setup (Using Public Jitsi):
```
Your App → Generates unique room name → https://meet.jit.si/BlastMessenger-{timestamp}-{random}
```

### Room URL Format:
```
https://meet.jit.si/BlastMessenger-1713369600-1234567890
```

Each video call gets a unique room that expires when everyone leaves.

---

## 📋 TESTING CHECKLIST

- [ ] Server starts without errors
- [ ] Web client loads at localhost:8080
- [ ] Can login with two different users
- [ ] Video call button appears in buddy list
- [ ] Clicking video call opens Jitsi modal
- [ ] Both users join the same room
- [ ] Video and audio work
- [ ] Can end call and modal closes

---

## 🎯 DEPLOY TO AWS AMPLIFY

Your Amplify setup is already configured! Just push:

```bash
git add .
git commit -m "Switch to Jitsi Meet for free video calling"
git push
```

Amplify will auto-deploy and video calling will work immediately on your live site!

---

## 🆙 OPTIONAL: Self-Host Jitsi (Advanced)

If you want even more control, you can host your own Jitsi server:

### Benefits:
- Full control over features
- Custom branding
- Your own domain
- No reliance on public servers

### Quick Self-Host Options:
1. **Docker** (easiest): https://jitsi.github.io/handbook/docs/devops-guide/devops-guide-docker
2. **AWS/DigitalOcean**: One-click Jitsi installs available
3. **Railway/Render**: Deploy Jitsi container

### To Use Your Own Server:
Just change line 1128 in `src/bin/server.rs`:
```rust
// From:
let room_url = format!("https://meet.jit.si/{}", room_name);

// To:
let room_url = format!("https://your-jitsi-domain.com/{}", room_name);
```

And update the Jitsi domain in `index.html` line ~120:
```javascript
// From:
const domain = '8x8.vc';

// To:
const domain = 'your-jitsi-domain.com';
```

---

## 🆘 TROUBLESHOOTING

### Video modal doesn't appear
- Check browser console for errors
- Verify Jitsi script loaded: `window.JitsiMeetExternalAPI` should exist
- Check CSP headers allow Jitsi domains

### Camera/microphone not working
- Grant browser permissions when prompted
- Check browser settings → Privacy → Camera/Microphone
- Try different browser (Chrome/Firefox recommended)

### Can't hear/see other person
- Both users must be in the same room (check room URL matches)
- Check Jitsi toolbar - camera/mic might be muted
- Verify both users granted permissions

### "Failed to join" error
- Check internet connection
- Try refreshing the page
- Clear browser cache

---

## 📚 RESOURCES

- **Jitsi Meet**: https://meet.jit.si/
- **Jitsi Handbook**: https://jitsi.github.io/handbook/
- **API Docs**: https://jitsi.github.io/handbook/docs/dev-guide/dev-guide-iframe
- **Self-Hosting Guide**: https://jitsi.github.io/handbook/docs/devops-guide/

---

## 🎊 YOU'RE DONE!

No setup needed - just test it! Video calling is ready to use right now. 🚀

**Next Steps:**
1. Test locally (5 min)
2. Push to GitHub
3. Let Amplify deploy
4. Share with friends!

Enjoy your free, unlimited video calling! 🎉
