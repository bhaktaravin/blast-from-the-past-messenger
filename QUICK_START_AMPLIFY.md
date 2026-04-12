# 🚀 Quick Start: Deploy to AWS Amplify

## ✅ Your code is ready!

Everything is configured and pushed to GitHub. Now just follow these simple steps:

## Step 1: Go to AWS Amplify Console

Open this link: **https://console.aws.amazon.com/amplify/**

(Make sure you're logged into your AWS account)

## Step 2: Create New App

1. Click the orange **"New app"** button (top right)
2. Select **"Host web app"**
3. Choose **"GitHub"** as your Git provider
4. Click **"Authorize AWS Amplify"** (if first time)
   - This lets Amplify access your GitHub repos

## Step 3: Select Your Repository

1. Find and select: **`blast-from-the-past-messenger`**
2. Select branch: **`main`**
3. Click **"Next"**

## Step 4: Configure Build Settings

You should see:
- **App name**: `blast-from-the-past-messenger` (you can change this)
- **Build and test settings**: ✅ Amplify detected `amplify.yml`

If you don't see the green checkmark:
- Make sure `amplify.yml` is in your repo root
- Click "Edit" and paste this:

```yaml
version: 1
frontend:
  phases:
    preBuild:
      commands:
        - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        - source $HOME/.cargo/env
        - rustup target add wasm32-unknown-unknown
        - cargo install --locked trunk
    build:
      commands:
        - trunk build --release --public-url /
  artifacts:
    baseDirectory: dist
    files:
      - '**/*'
  cache:
    paths:
      - ~/.cargo/**/*
      - target/**/*
```

Click **"Next"**

## Step 5: Review and Deploy

1. Review all settings
2. Click **"Save and deploy"**

## Step 6: Wait for Build ⏳

**First build takes 10-15 minutes** (installing Rust, compiling, etc.)

You'll see 4 phases:
1. ⏳ **Provision** (1 min) - Setting up build environment
2. ⏳ **Build** (8-12 min) - Compiling Rust → WASM
3. ⏳ **Deploy** (1 min) - Uploading to CDN
4. ⏳ **Verify** (30 sec) - Final checks

☕ Grab a coffee! Subsequent builds will be much faster (~5 min) thanks to caching.

## Step 7: Get Your URL 🎉

Once complete, you'll see:
- ✅ Green checkmark
- Your live URL: `https://main.d1234567890.amplifyapp.com`

Click the URL to open your app!

## Step 8: Test Your App

1. **Login screen** should load with themes
2. **Try logging in** (connects to your Railway server)
3. **Test features**:
   - Cycle through themes (button in top bar)
   - Send messages
   - Check sound effects
   - View buddy list

## 🎊 You're Live!

Your app is now deployed and will auto-update every time you push to GitHub!

---

## What Happens Next?

### Every time you push code:

```bash
git add .
git commit -m "Add new feature"
git push origin main
```

Amplify will:
1. Detect the push
2. Start a new build (5-10 min)
3. Deploy automatically
4. Update your live site

### Monitor builds:

- Go to Amplify Console
- Click your app
- View build history and logs

---

## Optional: Add Custom Domain

Want `chat.yourdomain.com` instead of the Amplify URL?

1. In Amplify Console, click **"Domain management"**
2. Click **"Add domain"**
3. Enter your domain
4. Follow DNS setup instructions
5. SSL certificate is automatic!

---

## Troubleshooting

### Build fails?
- Check build logs in Amplify Console
- Look for Rust compilation errors
- Verify `amplify.yml` is correct

### App won't load?
- Check browser console (F12)
- Verify Railway server is running
- Check WebSocket URL in code

### Need help?
- See [AMPLIFY_SETUP.md](./AMPLIFY_SETUP.md) for detailed guide
- Check [DEPLOYMENT_CHECKLIST.md](./DEPLOYMENT_CHECKLIST.md)

---

## Cost

### Free Tier (First 12 months):
- 1,000 build minutes/month
- 15 GB data transfer/month
- 5 GB storage/month

### After Free Tier:
- ~$2-5/month for typical usage
- $0.01/build minute
- $0.15/GB data transfer

---

## Quick Links

- **Amplify Console**: https://console.aws.amazon.com/amplify/
- **Your GitHub Repo**: https://github.com/bhaktaravin/blast-from-the-past-messenger
- **Railway Server**: https://railway.app/dashboard

---

## Success! 🎉

You now have:
- ✅ Auto-deploying web app
- ✅ Global CDN
- ✅ HTTPS enabled
- ✅ Continuous deployment
- ✅ Build caching

**Share your URL with friends and start chatting!**
