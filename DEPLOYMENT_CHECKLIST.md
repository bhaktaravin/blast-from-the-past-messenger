# 🚀 Deployment Checklist

## Pre-Deployment

- [ ] All code committed and pushed to GitHub
- [ ] `amplify.yml` file is in the root directory
- [ ] Web app builds locally: `trunk build --release`
- [ ] Server is running on Railway
- [ ] WebSocket URL is correct in code

## AWS Amplify Setup

### 1. Initial Setup (One-time)
- [ ] Go to [AWS Amplify Console](https://console.aws.amazon.com/amplify/)
- [ ] Click "New app" → "Host web app"
- [ ] Authorize GitHub access
- [ ] Select repository: `blast-from-the-past-messenger`
- [ ] Select branch: `main`
- [ ] Verify `amplify.yml` is detected
- [ ] App name: `blast-from-the-past-web`
- [ ] Click "Save and deploy"

### 2. First Build (10-15 minutes)
- [ ] Wait for "Provision" phase
- [ ] Wait for "Build" phase (Rust compilation)
- [ ] Wait for "Deploy" phase
- [ ] Wait for "Verify" phase
- [ ] Build successful ✅

### 3. Test Deployment
- [ ] Open the Amplify URL (e.g., `https://main.xxxxx.amplifyapp.com`)
- [ ] Login screen loads
- [ ] Themes work (cycle through them)
- [ ] Can connect to Railway server
- [ ] Can send/receive messages
- [ ] Sound effects work (if enabled)
- [ ] Buddy list displays
- [ ] All features functional

## Post-Deployment

### Optional: Custom Domain
- [ ] Go to Amplify Console → Domain management
- [ ] Click "Add domain"
- [ ] Enter domain: `chat.yourdomain.com`
- [ ] Follow DNS setup instructions
- [ ] Wait for SSL certificate (automatic)
- [ ] Verify domain works

### Optional: Branch Deployments
- [ ] Create `develop` branch
- [ ] Connect in Amplify Console
- [ ] Get staging URL: `https://develop.xxxxx.amplifyapp.com`
- [ ] Test features before merging to main

### Optional: Notifications
- [ ] Amplify Console → Notifications
- [ ] Add email for build notifications
- [ ] Get alerts on success/failure

## Ongoing Workflow

### Every Code Update
```bash
# 1. Make changes locally
# 2. Test locally
trunk serve

# 3. Commit and push
git add .
git commit -m "Add new feature"
git push origin main

# 4. Amplify auto-builds (5-10 min)
# 5. Check build status in Amplify Console
# 6. Test live site
```

### Monitor Builds
- [ ] Bookmark Amplify Console
- [ ] Check build logs if issues
- [ ] Monitor costs in AWS Billing

## Troubleshooting

### Build Fails
- [ ] Check build logs in Amplify Console
- [ ] Verify `amplify.yml` syntax
- [ ] Test build locally: `trunk build --release`
- [ ] Check Rust version compatibility
- [ ] Clear Amplify cache if needed

### App Not Loading
- [ ] Check browser console for errors
- [ ] Verify WASM files in dist/
- [ ] Check WebSocket connection
- [ ] Verify Railway server is running
- [ ] Check CORS settings

### WebSocket Issues
- [ ] Verify Railway URL is correct
- [ ] Check Railway server logs
- [ ] Ensure using `wss://` not `ws://`
- [ ] Test Railway server directly

## Cost Monitoring

### Free Tier (First 12 months)
- 1,000 build minutes/month
- 15 GB served/month
- 5 GB stored/month

### Check Usage
- [ ] AWS Billing Dashboard
- [ ] Amplify Console → Usage
- [ ] Set up billing alerts

### Expected Costs (After Free Tier)
- Builds: ~$1/month (10 builds)
- Data transfer: ~$1.50/month (10 GB)
- **Total**: ~$2-5/month

## Success Metrics

- [ ] Build time: 5-10 minutes (after first build)
- [ ] Deploy time: < 1 minute
- [ ] App loads in < 3 seconds
- [ ] WebSocket connects immediately
- [ ] No console errors
- [ ] All themes work
- [ ] Sound effects play
- [ ] Mobile responsive

## Documentation

- [ ] Update README with live URL
- [ ] Document any custom configuration
- [ ] Share URL with users
- [ ] Create user guide (optional)

## Security

- [ ] HTTPS enabled (automatic)
- [ ] Railway server has CORS configured
- [ ] No secrets in client code
- [ ] Environment variables secure
- [ ] Access logs enabled

## Next Steps

- [ ] Add analytics (Google Analytics, Plausible)
- [ ] Set up error tracking (Sentry)
- [ ] Create landing page
- [ ] Add user documentation
- [ ] Plan feature roadmap
- [ ] Gather user feedback

---

## Quick Reference

**Amplify Console**: https://console.aws.amazon.com/amplify/

**Your App URL**: `https://main.xxxxx.amplifyapp.com` (update after deployment)

**Railway Server**: `wss://blast-from-the-past-messenger-production.up.railway.app`

**Build Command**: `trunk build --release`

**Local Dev**: `trunk serve`

---

## Support Resources

- [Amplify Documentation](https://docs.amplify.aws/)
- [Trunk Documentation](https://trunkrs.dev/)
- [Railway Documentation](https://docs.railway.app/)
- [Your GitHub Repo](https://github.com/bhaktaravin/blast-from-the-past-messenger)

---

**Last Updated**: {{ date }}

**Status**: 🟢 Ready to deploy!
