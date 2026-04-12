# AWS Amplify Deployment Guide

## 🚀 Quick Setup (5 minutes!)

### Step 1: Push Your Code to GitHub

Make sure your latest code is on GitHub:

```bash
git add .
git commit -m "Add Amplify configuration"
git push origin main
```

### Step 2: Connect to AWS Amplify

1. **Go to AWS Amplify Console**
   - Visit: https://console.aws.amazon.com/amplify/
   - Click "New app" → "Host web app"

2. **Connect Your Repository**
   - Select "GitHub"
   - Click "Authorize AWS Amplify" (if first time)
   - Select your repository: `blast-from-the-past-messenger`
   - Select branch: `main`
   - Click "Next"

3. **Configure Build Settings**
   - App name: `blast-from-the-past-web`
   - Amplify will auto-detect the `amplify.yml` file ✅
   - Click "Next"

4. **Review and Deploy**
   - Review settings
   - Click "Save and deploy"

### Step 3: Wait for Build (10-15 minutes)

Amplify will:
1. ✅ Provision build environment
2. ✅ Install Rust and dependencies
3. ✅ Build your WASM app
4. ✅ Deploy to global CDN
5. ✅ Generate HTTPS URL

You'll get a URL like: `https://main.d1234567890.amplifyapp.com`

## 🎉 That's It!

Every time you push to GitHub, Amplify will automatically:
- Build your app
- Run tests (if you add them)
- Deploy to production
- Invalidate CDN cache

## 📝 What the amplify.yml Does

```yaml
preBuild:
  - Installs Rust toolchain
  - Adds WASM target
  - Installs trunk bundler

build:
  - Runs: trunk build --release
  - Outputs to: dist/

artifacts:
  - Serves everything in dist/
```

## 🔧 Configuration Options

### Environment Variables

Add these in Amplify Console → App Settings → Environment variables:

```
RUST_VERSION=stable
CARGO_TERM_COLOR=always
```

### Custom Domain

1. Go to Amplify Console → Domain management
2. Click "Add domain"
3. Enter your domain (e.g., `chat.yourdomain.com`)
4. Follow DNS setup instructions
5. SSL certificate is automatic! 🎉

### Branch Deployments

Want staging environment?

1. Go to Amplify Console → App settings → Branch settings
2. Connect `develop` branch
3. Get separate URL: `https://develop.d1234567890.amplifyapp.com`

### Build Notifications

Get notified on build status:

1. Go to Amplify Console → Notifications
2. Add email or SNS topic
3. Get alerts on build success/failure

## 💰 Cost Estimate

### Free Tier (First 12 months)
- 1,000 build minutes/month
- 15 GB served/month
- 5 GB stored/month

### After Free Tier
- **Build minutes**: $0.01/minute
  - ~10 min/build = $0.10/build
  - 10 builds/month = $1.00
- **Data transfer**: $0.15/GB
  - 10 GB/month = $1.50
- **Storage**: $0.023/GB/month
  - 100 MB = $0.002

**Total**: ~$2-5/month for moderate usage

## 🔍 Monitoring

### Build Logs
- View in Amplify Console → Build history
- See Rust compilation output
- Debug build failures

### Access Logs
- Amplify Console → Monitoring
- Request counts
- Error rates
- Geographic distribution

### Performance
- CloudWatch metrics
- Response times
- Cache hit rates

## 🐛 Troubleshooting

### Build Fails: "Rust not found"
**Solution**: Amplify.yml is correct, just wait for cache to clear

### Build Fails: "trunk not found"
**Solution**: Check cargo install command in amplify.yml

### WASM not loading
**Solution**: 
- Check browser console
- Verify dist/ folder has .wasm files
- Check MIME types (Amplify handles this automatically)

### WebSocket connection fails
**Solution**:
- Verify Railway server URL in code
- Check CORS settings on Railway
- Ensure using `wss://` not `ws://`

### Build takes too long (>15 min)
**Solution**:
- Cache is working after first build
- Subsequent builds: ~5 minutes
- Clear cache if needed: Amplify Console → Build settings → Clear cache

## 🚀 Advanced Features

### Preview Deployments

Every Pull Request gets its own URL!

1. Enable in Amplify Console → Previews
2. Create PR on GitHub
3. Get preview URL in PR comments
4. Test before merging

### Redirects and Rewrites

Create `public/_redirects` file:

```
# SPA fallback
/*    /index.html   200

# Custom redirects
/old-path  /new-path  301
```

### Custom Headers

Add to amplify.yml:

```yaml
customHeaders:
  - pattern: '**/*'
    headers:
      - key: 'Strict-Transport-Security'
        value: 'max-age=31536000; includeSubDomains'
      - key: 'X-Frame-Options'
        value: 'SAMEORIGIN'
      - key: 'X-Content-Type-Options'
        value: 'nosniff'
```

### Password Protection

Protect staging environment:

1. Amplify Console → Access control
2. Enable password protection
3. Set username/password
4. Only for non-production branches

## 📊 Performance Optimization

### Enable Compression
- Amplify automatically compresses assets
- Gzip for text files
- Brotli for modern browsers

### Cache Headers
Amplify sets optimal cache headers:
- HTML: 5 minutes
- JS/CSS/WASM: 1 year
- Images: 1 year

### CDN
- Global edge locations
- Automatic HTTPS
- HTTP/2 enabled

## 🔐 Security Best Practices

1. **HTTPS Only**: Enabled by default ✅
2. **Security Headers**: Add custom headers (see above)
3. **Access Control**: Use for staging environments
4. **Environment Variables**: Store secrets securely
5. **Branch Protection**: Require PR reviews

## 📱 Testing Your Deployment

### Local Testing
```bash
# Build locally first
trunk build --release

# Serve locally
trunk serve --release

# Test at http://localhost:8080
```

### Production Testing
```bash
# Get your Amplify URL
AMPLIFY_URL="https://main.d1234567890.amplifyapp.com"

# Test WebSocket connection
# Open browser console and check for connection
```

## 🔄 Deployment Workflow

```
1. Write code locally
2. Test with: trunk serve
3. Commit: git commit -m "Add feature"
4. Push: git push origin main
5. Amplify auto-builds (5-10 min)
6. Live at: https://your-app.amplifyapp.com
```

## 📚 Useful Commands

```bash
# View build logs
aws amplify list-apps
aws amplify get-app --app-id YOUR_APP_ID

# Trigger manual build
aws amplify start-job \
  --app-id YOUR_APP_ID \
  --branch-name main \
  --job-type RELEASE

# Delete app (careful!)
aws amplify delete-app --app-id YOUR_APP_ID
```

## 🎯 Next Steps

1. ✅ Deploy to Amplify
2. ✅ Test the live URL
3. ✅ Add custom domain (optional)
4. ✅ Set up branch deployments (optional)
5. ✅ Configure notifications
6. ✅ Monitor usage and costs

## 🆘 Support

- [AWS Amplify Documentation](https://docs.amplify.aws/)
- [Amplify Discord](https://discord.gg/amplify)
- [GitHub Issues](https://github.com/aws-amplify/amplify-hosting/issues)

## 🎊 Success Checklist

- [ ] Code pushed to GitHub
- [ ] Amplify app created
- [ ] First build successful
- [ ] App accessible via HTTPS URL
- [ ] WebSocket connects to Railway server
- [ ] Themes and features working
- [ ] Auto-deploy working on push

---

**Pro Tip**: Bookmark your Amplify Console URL for easy access to build logs and settings!

**Your Amplify Console**: https://console.aws.amazon.com/amplify/home
