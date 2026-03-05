# 🎯 Full Deployment Checklist - Option C

## ✅ STEP 1: Release Build (DONE!)

**Status:** ✅ Complete - Tag pushed!

**What's Happening:**
- GitHub Actions is building your apps
- Check progress: https://github.com/bhaktaravin/blast-from-the-past-messenger/actions
- Time: 5-10 minutes

**When Complete:**
Downloads available at:
https://github.com/bhaktaravin/blast-from-the-past-messenger/releases/tag/v1.0.0

You'll get:
- `blast-from-the-past-macos.dmg` (macOS installer)
- `blast-from-the-past-windows.zip` (Windows app)
- `blast-from-the-past-linux-x86_64.tar.gz` (Linux binary)

---

## 🚂 STEP 2: Deploy Server to Railway

### A. Create Railway Account (2 minutes)

1. Go to: https://railway.app
2. Click "Login" → "Login with GitHub"
3. Authorize Railway

### B. Create New Project (1 minute)

1. Click "New Project"
2. Select "Deploy from GitHub repo"
3. Choose: `blast-from-the-past-messenger`
4. Railway will start building!

### C. Add PostgreSQL (30 seconds)

1. In your project dashboard, click "+ New"
2. Select "Database"
3. Choose "Add PostgreSQL"
4. Done! `DATABASE_URL` is auto-set

### D. Configure Environment Variables (1 minute)

1. Click on your service (messenger-server)
2. Go to "Variables" tab
3. Add variable:
   ```
   BIND_ADDR = 0.0.0.0:$PORT
   ```
4. `DATABASE_URL` should already be there (auto-added)
5. `PORT` is automatically provided by Railway

### E. Deploy (automatic!)

Railway will:
- Detect it's a Rust project
- Use nixpacks.toml config
- Build the server
- Deploy automatically

**Time:** 5-10 minutes for first build

### F. Get Your Server URL

1. Go to "Settings" tab
2. Scroll to "Domains"
3. Click "Generate Domain"
4. You'll get: `your-app-name.up.railway.app`

**Save this URL!** You'll need it for the client.

---

## 🌐 STEP 3: Enable GitHub Pages (30 seconds)

1. Go to: https://github.com/bhaktaravin/blast-from-the-past-messenger/settings/pages
2. Under "Source", select: **GitHub Actions**
3. Click "Save"

**Web app will be live at:**
```
https://bhaktaravin.github.io/blast-from-the-past-messenger
```

Time: 5 minutes for first deploy

---

## 📝 STEP 4: Update Client Configuration

### Update Default Server URL

Edit `src/main.rs`:

Find this line (around line 184):
```rust
server_url: "wss://blast-from-the-past-messenger.fly.dev".to_string(),
```

Change to your Railway URL:
```rust
server_url: "wss://your-app-name.up.railway.app".to_string(),
```

### Commit and Push

```bash
cd ~/Code/blast-from-the-past-messenger
git add src/main.rs
git commit -m "Update server URL to Railway"
git push origin main
```

This will:
- Auto-deploy web version with new URL
- Be included in next release

---

## ✅ Verification Checklist

### GitHub Actions Release
- [ ] Go to https://github.com/bhaktaravin/blast-from-the-past-messenger/actions
- [ ] See "Release Builds" workflow running
- [ ] Wait for green checkmark
- [ ] Check releases page for downloads

### Railway Server
- [ ] Railway project created
- [ ] PostgreSQL addon added
- [ ] Environment variables set
- [ ] Server deployed successfully
- [ ] Domain generated
- [ ] Server responding (check logs)

### GitHub Pages
- [ ] GitHub Pages enabled in settings
- [ ] Web Deploy workflow running
- [ ] Site accessible at github.io URL

### Client Update
- [ ] Server URL updated in code
- [ ] Changes committed and pushed
- [ ] Web version auto-deploys

---

## 🎯 Timeline

| Task | Time | Status |
|------|------|--------|
| Create release tag | ✅ Done | Complete |
| GitHub Actions build | 5-10 min | In Progress |
| Railway signup | 2 min | Next |
| Railway deploy | 5-10 min | Next |
| Enable GitHub Pages | 30 sec | Next |
| Update client URL | 2 min | Next |
| **Total** | ~25 min | - |

---

## 🔗 Important URLs

### Your GitHub
- **Repo:** https://github.com/bhaktaravin/blast-from-the-past-messenger
- **Actions:** https://github.com/bhaktaravin/blast-from-the-past-messenger/actions
- **Releases:** https://github.com/bhaktaravin/blast-from-the-past-messenger/releases
- **Settings:** https://github.com/bhaktaravin/blast-from-the-past-messenger/settings

### Railway
- **Dashboard:** https://railway.app/dashboard
- **Docs:** https://docs.railway.app

### After Deployment
- **Web App:** https://bhaktaravin.github.io/blast-from-the-past-messenger
- **Server:** https://your-app-name.up.railway.app (you'll get this)
- **Downloads:** https://github.com/bhaktaravin/blast-from-the-past-messenger/releases

---

## 🚨 Troubleshooting

### GitHub Actions Fails?
- Check the logs in Actions tab
- Most common: Build timeout (try again)
- Contact me if issues persist

### Railway Build Fails?
- Check build logs in Railway dashboard
- Ensure `server` feature exists in Cargo.toml
- Verify DATABASE_URL is set

### Can't Connect to Server?
- Check Railway logs for errors
- Verify BIND_ADDR uses $PORT variable
- Ensure server is listening on 0.0.0.0

### Web Version Not Working?
- Wait 5 minutes after enabling Pages
- Hard refresh browser (Cmd+Shift+R / Ctrl+Shift+R)
- Check GitHub Actions for web-deploy status

---

## 🎉 Success Indicators

You'll know everything is working when:

✅ **Release page** has 3 downloadable files
✅ **Railway dashboard** shows green "Active" status
✅ **Railway logs** show "Server listening on..."
✅ **Web app** loads at github.io URL
✅ **Clients can connect** to Railway server

---

## 📞 Next Steps After Deployment

1. **Download and test** each platform build
2. **Connect client** to Railway server
3. **Share web URL** with friends
4. **Monitor Railway** dashboard for usage
5. **Celebrate!** 🎊

---

## 🎁 Bonus: Custom Domain (Optional)

### For Web App (GitHub Pages)
1. Buy domain (Namecheap, Cloudflare, etc.)
2. Add CNAME record: `messenger → bhaktaravin.github.io`
3. In GitHub repo settings, add custom domain
4. Wait for SSL certificate

### For Server (Railway)
1. In Railway → Settings → Domains
2. Click "Custom Domain"
3. Add CNAME record: `api → your-app.up.railway.app`
4. SSL automatically provisioned

---

**Ready?** Start with Railway signup now! 🚂

1. Open: https://railway.app
2. Click "Login with GitHub"
3. Follow steps B-F above

You got this! 🚀
