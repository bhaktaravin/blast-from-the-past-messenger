# Deploying Web Version to AWS S3 + CloudFront

## Overview
This guide shows how to host your WASM web app on AWS S3 with CloudFront CDN for fast, cheap, and scalable hosting.

**Estimated Cost**: $1-5/month for moderate traffic

## Prerequisites
- AWS Account
- AWS CLI installed (`brew install awscli` on macOS)
- Your web app built (`trunk build --release`)

## Step 1: Build the Web App

```bash
# Install trunk if you haven't
cargo install trunk

# Build for production
trunk build --release --public-url /

# Your files will be in ./dist/
```

## Step 2: Create S3 Bucket

```bash
# Set your bucket name (must be globally unique)
BUCKET_NAME="blast-from-the-past-web"

# Create bucket
aws s3 mb s3://$BUCKET_NAME --region us-east-1

# Enable static website hosting
aws s3 website s3://$BUCKET_NAME \
  --index-document index.html \
  --error-document index.html
```

## Step 3: Configure Bucket Policy

Create a file `bucket-policy.json`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "PublicReadGetObject",
      "Effect": "Allow",
      "Principal": "*",
      "Action": "s3:GetObject",
      "Resource": "arn:aws:s3:::blast-from-the-past-web/*"
    }
  ]
}
```

Apply the policy:

```bash
aws s3api put-bucket-policy \
  --bucket $BUCKET_NAME \
  --policy file://bucket-policy.json
```

## Step 4: Upload Your Files

```bash
# Upload all files from dist/
aws s3 sync ./dist/ s3://$BUCKET_NAME/ \
  --delete \
  --cache-control "public, max-age=31536000" \
  --exclude "index.html"

# Upload index.html with shorter cache (for updates)
aws s3 cp ./dist/index.html s3://$BUCKET_NAME/index.html \
  --cache-control "public, max-age=300"

# Set correct MIME types for WASM
aws s3 cp ./dist/ s3://$BUCKET_NAME/ \
  --recursive \
  --exclude "*" \
  --include "*.wasm" \
  --content-type "application/wasm" \
  --metadata-directive REPLACE
```

## Step 5: Create CloudFront Distribution

```bash
# Create distribution (this takes 10-15 minutes)
aws cloudfront create-distribution \
  --origin-domain-name $BUCKET_NAME.s3-website-us-east-1.amazonaws.com \
  --default-root-object index.html
```

Or use the AWS Console:
1. Go to CloudFront → Create Distribution
2. **Origin Domain**: Select your S3 bucket
3. **Viewer Protocol Policy**: Redirect HTTP to HTTPS
4. **Allowed HTTP Methods**: GET, HEAD, OPTIONS
5. **Compress Objects**: Yes
6. **Default Root Object**: index.html
7. Create Distribution

## Step 6: Configure Custom Domain (Optional)

### Using Route 53:

1. **Get CloudFront domain name** (e.g., `d1234567890.cloudfront.net`)

2. **Create Route 53 Record**:
```bash
# In Route 53, create an A record (Alias)
# Point to your CloudFront distribution
```

3. **Add SSL Certificate** (AWS Certificate Manager):
   - Request certificate for your domain
   - Add CNAME records for validation
   - Attach to CloudFront distribution

## Step 7: Deployment Script

Create `deploy-web.sh`:

```bash
#!/bin/bash
set -e

BUCKET_NAME="blast-from-the-past-web"
DISTRIBUTION_ID="YOUR_CLOUDFRONT_ID"  # Get from CloudFront console

echo "🔨 Building web app..."
trunk build --release --public-url /

echo "📤 Uploading to S3..."
aws s3 sync ./dist/ s3://$BUCKET_NAME/ \
  --delete \
  --cache-control "public, max-age=31536000" \
  --exclude "index.html"

aws s3 cp ./dist/index.html s3://$BUCKET_NAME/index.html \
  --cache-control "public, max-age=300"

# Fix WASM MIME type
aws s3 cp ./dist/ s3://$BUCKET_NAME/ \
  --recursive \
  --exclude "*" \
  --include "*.wasm" \
  --content-type "application/wasm" \
  --metadata-directive REPLACE

echo "🔄 Invalidating CloudFront cache..."
aws cloudfront create-invalidation \
  --distribution-id $DISTRIBUTION_ID \
  --paths "/*"

echo "✅ Deployment complete!"
echo "🌐 Your app will be live in a few minutes"
```

Make it executable:
```bash
chmod +x deploy-web.sh
```

## Step 8: Deploy Updates

```bash
./deploy-web.sh
```

## Cost Breakdown

### S3 Storage
- First 50 TB: $0.023/GB/month
- For 100MB app: ~$0.002/month

### S3 Requests
- GET requests: $0.0004 per 1,000
- 10,000 requests: ~$0.004

### CloudFront
- First 10 TB: $0.085/GB
- 1 GB transfer: ~$0.085
- **Total for 1,000 users/month**: ~$2-5

### Free Tier (First 12 months)
- S3: 5GB storage, 20,000 GET requests
- CloudFront: 50GB data transfer out

## Alternative: AWS Amplify (Even Easier!)

If you want automatic deployments from GitHub:

### 1. Push to GitHub
```bash
git push origin main
```

### 2. Connect to Amplify
1. Go to AWS Amplify Console
2. Click "New App" → "Host web app"
3. Connect your GitHub repo
4. **Build settings**:
```yaml
version: 1
frontend:
  phases:
    preBuild:
      commands:
        - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        - source $HOME/.cargo/env
        - rustup target add wasm32-unknown-unknown
        - cargo install trunk
    build:
      commands:
        - trunk build --release
  artifacts:
    baseDirectory: dist
    files:
      - '**/*'
  cache:
    paths:
      - ~/.cargo/**/*
      - target/**/*
```

5. Deploy!

**Amplify Benefits**:
- Auto-deploy on git push
- Preview deployments for PRs
- Built-in CDN and HTTPS
- Custom domains easy
- **Cost**: ~$0.15/GB + $0.01/build minute

## Connecting to Your Railway Server

Your web app will connect to your Railway WebSocket server:

```rust
// In your web build, update the server URL
let server_url = "wss://blast-from-the-past-messenger-production.up.railway.app";
```

Make sure your Railway server has CORS configured to allow your CloudFront/S3 domain.

## Monitoring

### CloudWatch Metrics
- S3 bucket metrics
- CloudFront request counts
- Error rates

### CloudFront Reports
- Popular objects
- Top referrers
- Usage reports

## Security Best Practices

1. **Enable CloudFront HTTPS only**
2. **Use S3 bucket policies** (not public ACLs)
3. **Enable CloudFront logging**
4. **Set up AWS WAF** (optional, for DDoS protection)
5. **Use CloudFront signed URLs** (if you want private content)

## Troubleshooting

### WASM not loading
- Check MIME type: `application/wasm`
- Check CORS headers
- Check browser console for errors

### Updates not showing
- Invalidate CloudFront cache
- Check cache-control headers
- Hard refresh browser (Cmd+Shift+R)

### WebSocket connection fails
- Check Railway server is running
- Verify WebSocket URL is correct
- Check CORS settings on server

## Next Steps

1. Set up custom domain
2. Add SSL certificate
3. Configure CI/CD with GitHub Actions
4. Set up monitoring and alerts
5. Add analytics (Google Analytics, Plausible, etc.)

## Useful Commands

```bash
# Check bucket contents
aws s3 ls s3://$BUCKET_NAME/

# Download from bucket
aws s3 sync s3://$BUCKET_NAME/ ./downloaded/

# Delete bucket (careful!)
aws s3 rb s3://$BUCKET_NAME --force

# List CloudFront distributions
aws cloudfront list-distributions

# Get distribution details
aws cloudfront get-distribution --id YOUR_DISTRIBUTION_ID
```

## Resources

- [AWS S3 Static Website Hosting](https://docs.aws.amazon.com/AmazonS3/latest/userguide/WebsiteHosting.html)
- [CloudFront Documentation](https://docs.aws.amazon.com/cloudfront/)
- [AWS Amplify Hosting](https://docs.amplify.aws/guides/hosting/)
- [Trunk WASM Bundler](https://trunkrs.dev/)
