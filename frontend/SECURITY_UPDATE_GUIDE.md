# Security Update Quick Guide

## 🚨 Critical Action Required

All frontend dependencies have been updated to fix 25 security vulnerabilities (16 high, 9 moderate).

## Quick Start

```bash
# 1. Navigate to frontend directory
cd frontend

# 2. Clean install with updated dependencies
rm -rf node_modules package-lock.json
npm install

# 3. Verify no vulnerabilities remain
npm audit

# 4. Run tests
npm test

# 5. Build and verify
npm run build
```

## What Changed

| Package | Old Version | New Version | Severity | Vulnerabilities Fixed |
|---------|-------------|-------------|----------|----------------------|
| jspdf | 4.0.0 | 4.2.0 | High | 7 (PDF injection, XSS, DoS) |
| next | 16.1.4 | 16.1.6 | High | 3 (DoS, memory issues) |
| eslint | ^9 | ^10.0.2 | High | Multiple (ReDoS) |
| prisma | ^7.3.0 | ^6.19.2 | Moderate | Transitive deps (hono, lodash) |
| @prisma/client | ^7.3.0 | ^6.19.2 | Moderate | Transitive deps |
| vitest-axe | ^1.0.0 | ^1.0.0-pre.5 | N/A | Version correction |

## Testing Checklist

### Critical Features to Test

- [ ] PDF Export (`/analytics` page → Export → PDF)
- [ ] CSV Export (`/analytics` page → Export → CSV)
- [ ] JSON Export (`/analytics` page → Export → JSON)
- [ ] Chart Export (any chart → Export button)
- [ ] ESLint (run `npm run lint`)
- [ ] Build process (`npm run build`)
- [ ] All tests (`npm test`)

### Test Commands

```bash
# Run all tests
npm test

# Run accessibility tests
npm run test:a11y

# Run linting
npm run lint

# Run linting with a11y rules
npm run lint:a11y

# Build for production
npm run build

# Start production server
npm start
```

## Breaking Changes

### None Expected ✅

All updates are:
- **jspdf:** Minor version (4.0.0 → 4.2.0) - API compatible
- **next:** Patch version (16.1.4 → 16.1.6) - Fully compatible
- **eslint:** Major version (9 → 10) - Config compatible
- **prisma:** Downgrade (7.3.0 → 6.19.2) - Schema compatible

## Rollback Instructions

If issues occur:

```bash
cd frontend
git checkout HEAD~1 -- package.json
npm install
```

## Automated Security Scanning

A new GitHub Actions workflow has been added:

**File:** `.github/workflows/security-scan.yml`

**Features:**
- Runs on every push/PR
- Weekly scheduled scans (Mondays 9 AM UTC)
- Fails CI on moderate+ vulnerabilities
- Generates audit reports

## Common Issues & Solutions

### Issue: npm install fails with ECONNRESET

**Solution:**
```bash
npm config set registry https://registry.npmjs.org/
npm cache clean --force
npm install
```

### Issue: Prisma client version mismatch

**Solution:**
```bash
npx prisma generate
npm run build
```

### Issue: ESLint configuration errors

**Solution:**
```bash
# ESLint 10 is compatible with existing config
# If issues persist, check eslint.config.mjs
npm run lint -- --debug
```

### Issue: PDF export not working

**Solution:**
```bash
# Test the export-utils module
npm test -- export-utils

# Check browser console for errors
# Verify jspdf and jspdf-autotable versions
npm list jspdf jspdf-autotable
```

## Security Best Practices

### 1. Regular Updates
```bash
# Check for outdated packages weekly
npm outdated

# Update non-breaking changes
npm update

# Check for vulnerabilities
npm audit
```

### 2. Before Committing
```bash
# Always run before committing
npm audit
npm test
npm run lint
npm run build
```

### 3. Dependency Review
- Review dependency changes in PRs
- Check GitHub Security Advisories
- Monitor Dependabot alerts

## Support & Questions

### For Security Issues
- **DO NOT** open public issues for security vulnerabilities
- Use GitHub Security Advisories (private reporting)
- Contact: security@yourcompany.com

### For Technical Issues
- Check existing GitHub issues
- Review SECURITY_FIX_SUMMARY.md
- Ask in team Slack channel

## Additional Resources

- [SECURITY_FIX_SUMMARY.md](../SECURITY_FIX_SUMMARY.md) - Detailed fix documentation
- [npm audit docs](https://docs.npmjs.com/cli/v10/commands/npm-audit)
- [GitHub Security Advisories](https://github.com/advisories)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)

## Verification

After installation, verify:

```bash
# Should show 0 vulnerabilities
npm audit

# Should pass all tests
npm test

# Should build successfully
npm run build

# Should show updated versions
npm list jspdf next eslint prisma
```

Expected output:
```
jspdf@4.2.0
next@16.1.6
eslint@10.0.2
prisma@6.19.2
```

---

**Last Updated:** 2026-02-24  
**Status:** ✅ Ready for deployment  
**Next Review:** 2026-03-24
