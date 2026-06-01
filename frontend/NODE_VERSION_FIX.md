# Node.js Version Compatibility Issue

## Problem
The frontend dependencies require Node.js v20.19+, v22.12+, or v24.0+, but the CI environment (and potentially local development) is running Node.js v18.20.8.

### Error Messages
```
npm warn EBADENGINE Unsupported engine {
npm warn EBADENGINE   package: '@prisma/client@7.3.0',
npm warn EBADENGINE   required: { node: '^20.19 || ^22.12 || >=24.0' },
npm warn EBADENGINE   current: { node: 'v18.20.8', npm: '10.8.2' }
npm warn EBADENGINE }
```

### Affected Packages
- `@prisma/client@7.3.0` - Requires Node.js 20.19+, 22.12+, or 24.0+
- `next@16.1.4` - Requires Node.js 20.9.0+
- `prisma@7.3.0` - Requires Node.js 20.19+, 22.12+, or 24.0+

## Solutions

### Option 1: Upgrade Node.js (Recommended)
Upgrade to a compatible Node.js version:

```bash
# Using nvm (Node Version Manager)
nvm install 22.12.0
nvm use 22.12.0

# Or using Homebrew (macOS)
brew install node@22

# Or using apt (Ubuntu/Debian)
sudo apt-get install nodejs=22.12.0-1nodesource1
```

### Option 2: Downgrade Dependencies (Not Recommended)
If you must use Node.js v18, downgrade the dependencies:

```bash
npm install @prisma/client@6.x prisma@6.x next@15.x
```

**Note:** This may introduce compatibility issues with other parts of the codebase.

### Option 3: Use Docker
Create a Docker container with the correct Node.js version:

```dockerfile
FROM node:22.12.0-alpine

WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build
```

## For CI/CD Environments

### GitHub Actions
Update your workflow file to use Node.js 22:

```yaml
- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: '22.12.0'
    cache: 'npm'
```

### GitLab CI
Update your `.gitlab-ci.yml`:

```yaml
image: node:22.12.0-alpine

stages:
  - build

build:
  stage: build
  script:
    - npm ci
    - npm run build
```

## Verification

After upgrading Node.js, verify the version:

```bash
node --version  # Should be v22.12.0 or higher
npm --version   # Should be 10.x or higher
```

Then reinstall dependencies:

```bash
rm -rf node_modules package-lock.json
npm install
npm run build
```

## Current Status

- **Backend**: ✅ Compiles successfully with Rust 1.x
- **Frontend**: ⚠️ Requires Node.js v20.19+ or higher
- **Database**: ✅ SQLite configured and migrations ready
- **Cache**: ✅ Redis caching layer implemented

## Next Steps

1. Upgrade Node.js to v22.12.0 or later
2. Run `npm install` to update dependencies
3. Run `npm run build` to verify the build succeeds
4. Deploy with confidence
