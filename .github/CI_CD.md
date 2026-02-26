# CI/CD Documentation

This document explains the automated checks and workflows configured for Stellar Insights.

---

## 🔄 Workflows Overview

### 1. Backend CI (`backend.yml`)
**Triggers:** Push/PR to `main` or `develop` with backend changes

**Jobs:**
- **Test** - Runs all backend tests with PostgreSQL
  - Formatting check (`cargo fmt`)
  - Linting (`cargo clippy`)
  - Build verification
  - Unit & integration tests
  
- **Security Audit** - Checks for known vulnerabilities
  - Uses `cargo-audit`
  - Scans dependencies
  
- **Build Release** - Creates production binary
  - Optimized release build
  - Uploads artifact for deployment

**Status Badge:**
```markdown
![Backend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Backend%20CI/badge.svg)
```

---

### 2. Frontend CI (`frontend.yml`)
**Triggers:** Push/PR to `main` or `develop` with frontend changes

**Jobs:**
- **Test & Lint** - Validates frontend code
  - ESLint checks
  - TypeScript type checking
  - Build verification
  - Tests on Node 18.x and 20.x
  
- **Build Production** - Creates production build
  - Next.js optimization
  - Uploads build artifact

**Status Badge:**
```markdown
![Frontend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Frontend%20CI/badge.svg)
```

---

### 3. Smart Contracts CI (`contracts.yml`)
**Triggers:** Push/PR to `main` or `develop` with contract changes

**Jobs:**
- **Test Contracts** - Validates Soroban contracts
  - Formatting check
  - Clippy linting
  - Contract tests
  
- **Build WASM** - Compiles to WebAssembly
  - Builds all contracts
  - Uploads WASM artifacts
  
- **Contract Size** - Monitors binary sizes
  - Reports contract sizes
  - Helps track bloat

**Status Badge:**
```markdown
![Contracts CI](https://github.com/Ndifreke000/stellar-insights/workflows/Smart%20Contracts%20CI/badge.svg)
```

---

### 4. Full Stack CI (`full-stack.yml`)
**Triggers:** Push/PR to `main` branch

**Jobs:**
- **Integration Test** - End-to-end testing
  - Starts backend server
  - Tests API endpoints
  - Builds frontend
  - Verifies integration
  
- **Code Quality** - Repository health checks
  - Validates commit messages (conventional commits)
  - Checks for large files (>1MB)
  - Scans for potential secrets
  
- **Documentation** - Ensures docs are up-to-date
  - Verifies README exists
  - Checks for broken links

**Status Badge:**
```markdown
![Full Stack CI](https://github.com/Ndifreke000/stellar-insights/workflows/Full%20Stack%20CI/badge.svg)
```

---

## 📊 Status Badges

Add these to your README.md:

```markdown
![Backend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Backend%20CI/badge.svg)
![Frontend CI](https://github.com/Ndifreke000/stellar-insights/workflows/Frontend%20CI/badge.svg)
![Contracts CI](https://github.com/Ndifreke000/stellar-insights/workflows/Smart%20Contracts%20CI/badge.svg)
![Full Stack CI](https://github.com/Ndifreke000/stellar-insights/workflows/Full%20Stack%20CI/badge.svg)
```

---

## 🚦 Workflow Triggers

### Automatic Triggers
- **Push to main/develop** - Runs relevant workflows
- **Pull Request** - Runs all checks before merge
- **Path filters** - Only runs when relevant files change

### Manual Triggers
You can manually trigger workflows from GitHub Actions tab:
1. Go to Actions tab
2. Select workflow
3. Click "Run workflow"

---

## ✅ Required Checks

Before merging a PR, these checks must pass:

### Backend
- [ ] Code formatting (rustfmt)
- [ ] Linting (clippy with no warnings)
- [ ] All tests pass
- [ ] Security audit clean
- [ ] Release build succeeds

### Frontend
- [ ] ESLint passes
- [ ] TypeScript compiles
- [ ] Production build succeeds
- [ ] Tests on Node 18 & 20

### Contracts
- [ ] Code formatting
- [ ] Clippy passes
- [ ] All contract tests pass
- [ ] WASM builds successfully

### Full Stack
- [ ] Integration tests pass
- [ ] Commit messages valid
- [ ] No large files
- [ ] No secrets detected
- [ ] Documentation complete

---

## 🔧 Local Testing

Run the same checks locally before pushing:

### Backend
```bash
cd backend

# Format check
cargo fmt --check

# Linting
cargo clippy -- -D warnings

# Tests
cargo test

# Security audit
cargo install cargo-audit
cargo audit

# Build
cargo build --release
```

### Frontend
```bash
cd frontend

# Lint
npm run lint

# Type check
npx tsc --noEmit

# Build
npm run build
```

### Contracts
```bash
cd contracts

# Format
cargo fmt --check

# Lint
cargo clippy --all-targets -- -D warnings

# Test
cargo test

# Build WASM
cargo build --target wasm32-unknown-unknown --release
```

---

## 🐛 Troubleshooting

### Workflow Fails on Formatting
```bash
# Auto-fix formatting
cargo fmt
git add .
git commit -m "style: fix formatting"
```

### Clippy Warnings
```bash
# See all warnings
cargo clippy

# Auto-fix some issues
cargo clippy --fix
```

### TypeScript Errors
```bash
# Check types
npx tsc --noEmit

# Fix in your editor or manually
```

### Test Failures
```bash
# Run tests locally
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture
```

---

## 📈 Monitoring

### GitHub Actions Dashboard
- View all workflow runs: `https://github.com/Ndifreke000/stellar-insights/actions`
- Filter by workflow, branch, or status
- Download artifacts from successful runs

### Artifacts
Workflows upload these artifacts:
- **backend-binary** - Compiled Rust binary (7 days)
- **frontend-build** - Next.js build (7 days)
- **contract-wasm** - Compiled contracts (7 days)

---

## 🔐 Secrets Management

Required secrets (set in GitHub Settings > Secrets):
- None currently required for CI
- Add deployment secrets when needed

---

## 🚀 Deployment

### Backend Deployment
```bash
# Download artifact from successful workflow
# Or build locally:
cd backend
cargo build --release

# Deploy binary to server
scp target/release/backend user@server:/path/to/deploy
```

### Frontend Deployment
```bash
# Vercel (recommended)
vercel deploy

# Or manual:
cd frontend
npm run build
# Upload .next folder to hosting
```

### Contract Deployment
```bash
# Download WASM from workflow
# Or build locally:
cd contracts/snapshot-contract
cargo build --target wasm32-unknown-unknown --release

# Deploy to Stellar
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/snapshot_contract.wasm \
  --source YOUR_SECRET_KEY \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; September 2015"
```

---

## 📝 Adding New Checks

### Add a New Workflow
1. Create `.github/workflows/your-workflow.yml`
2. Define triggers and jobs
3. Test locally first
4. Commit and push

### Add to Existing Workflow
1. Edit relevant workflow file
2. Add new step or job
3. Test changes on a branch
4. Merge when working

---

## 🎯 Best Practices

1. **Run checks locally** before pushing
2. **Keep workflows fast** - use caching
3. **Fail fast** - run quick checks first
4. **Use matrix builds** for multiple versions
5. **Cache dependencies** to speed up builds
6. **Upload artifacts** for debugging
7. **Add status badges** to README

---

## 📚 Resources

- [GitHub Actions Docs](https://docs.github.com/en/actions)
- [Rust CI Best Practices](https://doc.rust-lang.org/cargo/guide/continuous-integration.html)
- [Next.js CI/CD](https://nextjs.org/docs/deployment)
- [Soroban CLI](https://soroban.stellar.org/docs/reference/soroban-cli)

---

**Last Updated:** January 26, 2026
