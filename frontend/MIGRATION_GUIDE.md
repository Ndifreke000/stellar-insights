# Security Update Migration Guide

## Overview

This guide helps you migrate from the vulnerable dependency versions to the secure versions.

## Version Changes Summary

| Package | Old Version | New Version | Breaking Changes |
|---------|-------------|-------------|------------------|
| jspdf | 4.0.0 | 4.2.0 | ❌ None |
| next | 16.1.4 | 16.1.6 | ❌ None |
| eslint | ^9 | ^10.0.2 | ⚠️ Minor config changes |
| prisma | ^7.3.0 | ^6.19.2 | ⚠️ Downgrade (see notes) |
| @prisma/client | ^7.3.0 | ^6.19.2 | ⚠️ Downgrade (see notes) |
| vitest-axe | ^1.0.0 | ^1.0.0-pre.5 | ❌ None |

## Migration Steps

### Step 1: Backup Current State

```bash
cd frontend

# Backup package files
cp package.json package.json.backup
cp package-lock.json package-lock.json.backup

# Backup node_modules (optional, for quick rollback)
# tar -czf node_modules.backup.tar.gz node_modules
```

### Step 2: Update Dependencies

#### Option A: Automated (Recommended)

```bash
# Linux/Mac
./update-dependencies.sh

# Windows PowerShell
.\update-dependencies.ps1
```

#### Option B: Manual

```bash
# Clean install
rm -rf node_modules package-lock.json
npm install

# Verify
npm audit
npm test
npm run build
```

### Step 3: Handle Breaking Changes

#### ESLint 10 Migration

ESLint 10 is mostly compatible with ESLint 9 configs, but check for:

**1. Flat Config (if using eslint.config.js):**

```javascript
// eslint.config.mjs - Should work as-is
import eslintConfigNext from 'eslint-config-next';

export default [
  ...eslintConfigNext,
  // Your custom rules
];
```

**2. Plugin Changes:**

Most plugins are compatible. If you see errors:

```bash
# Update ESLint plugins
npm update eslint-plugin-jsx-a11y
npm update eslint-config-next
```

**3. Rule Changes:**

Check for deprecated rules:

```bash
npm run lint -- --debug
```

#### Prisma 6.19.2 Downgrade

**Why downgrade?**
- Prisma 7.x includes vulnerable transitive dependencies (hono, lodash)
- Prisma 6.19.2 is stable and secure
- No schema changes required

**Migration steps:**

```bash
# 1. Regenerate Prisma client
npx prisma generate

# 2. Verify schema compatibility
npx prisma validate

# 3. Test database connection
npx prisma db pull --force

# 4. Run migrations if needed
npx prisma migrate dev
```

**Prisma 7 → 6 Compatibility:**

✅ **Compatible:**
- Schema syntax
- Query API
- Migrations
- Prisma Studio
- Most features

⚠️ **Check if you use:**
- TypedSQL (Prisma 7 feature - not available in 6.x)
- New Prisma 7 preview features
- Prisma Accelerate (check compatibility)

**If you need Prisma 7 features:**

Wait for Prisma 7.x to fix transitive dependencies, or:

```bash
# Override vulnerable dependencies (advanced)
npm install --save-dev hono@latest
```

### Step 4: Update Code (if needed)

#### jsPDF 4.0.0 → 4.2.0

No code changes required. API is fully compatible.

**Verify your usage:**

```typescript
// ✅ All these patterns work in 4.2.0
import jsPDF from "jspdf";
import autoTable from "jspdf-autotable";

const doc = new jsPDF();
doc.setFontSize(18);
doc.text("Title", 14, 22);
doc.setTextColor(100);

autoTable(doc, {
  head: [headers],
  body: data,
  startY: 44,
  theme: "grid",
  headStyles: { fillColor: [59, 130, 246] },
  styles: { fontSize: 8 },
});

doc.save("report.pdf");
```

#### Next.js 16.1.4 → 16.1.6

No code changes required. Patch version with security fixes.

**Verify your usage:**

```typescript
// ✅ All Next.js features work as before
import { NextRequest, NextResponse } from 'next/server';
import Image from 'next/image';
import Link from 'next/link';

// App Router, Server Components, etc. - all compatible
```

### Step 5: Test Everything

Follow the [TESTING_CHECKLIST.md](./TESTING_CHECKLIST.md) for comprehensive testing.

**Quick smoke test:**

```bash
# Run tests
npm test

# Build project
npm run build

# Start dev server
npm run dev

# Test in browser:
# 1. Navigate to http://localhost:3000
# 2. Test PDF export
# 3. Check console for errors
```

## Common Migration Issues

### Issue 1: Prisma Client Version Mismatch

**Error:**
```
Prisma Client version mismatch
Expected: 7.3.0
Actual: 6.19.2
```

**Solution:**
```bash
npx prisma generate
npm run build
```

### Issue 2: ESLint Configuration Errors

**Error:**
```
ESLint configuration is invalid
```

**Solution:**
```bash
# Check config syntax
npm run lint -- --debug

# If needed, update config
npx eslint --init
```

### Issue 3: jsPDF Type Errors

**Error:**
```
Type 'jsPDF' is not assignable to type...
```

**Solution:**
```bash
# Update TypeScript types
npm install --save-dev @types/jspdf@latest

# Or add type assertion
const doc = new jsPDF() as any;
```

### Issue 4: Build Failures

**Error:**
```
Module not found: Can't resolve 'jspdf'
```

**Solution:**
```bash
# Clear Next.js cache
rm -rf .next

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install

# Rebuild
npm run build
```

### Issue 5: Test Failures

**Error:**
```
Cannot find module 'vitest-axe'
```

**Solution:**
```bash
# Clear test cache
rm -rf node_modules/.cache

# Reinstall
npm install

# Regenerate test snapshots if needed
npm test -- -u
```

## API Changes Reference

### jsPDF 4.0.0 → 4.2.0

**No breaking changes.** All APIs remain the same.

**New features in 4.2.0:**
- Enhanced security (fixes 7 vulnerabilities)
- Improved PDF generation performance
- Better error handling

**Deprecated (but still work):**
- None

### Next.js 16.1.4 → 16.1.6

**No breaking changes.** Patch release with security fixes.

**Fixed issues:**
- DoS via Image Optimizer
- HTTP request deserialization DoS
- Memory consumption issues

### ESLint 9 → 10

**Minor breaking changes:**

1. **Flat config is now default:**
   ```javascript
   // Old: .eslintrc.json
   // New: eslint.config.mjs (preferred)
   ```

2. **Some rules updated:**
   - Check `npm run lint` output for warnings

3. **Plugin compatibility:**
   - Most plugins work as-is
   - Update if you see errors

### Prisma 7.3.0 → 6.19.2

**Breaking changes:**

1. **TypedSQL not available:**
   ```typescript
   // ❌ Not available in 6.x
   import { sql } from '@prisma/client/sql';
   
   // ✅ Use raw queries instead
   await prisma.$queryRaw`SELECT * FROM users`;
   ```

2. **Some preview features unavailable:**
   ```prisma
   // Check your schema for Prisma 7-only features
   generator client {
     provider = "prisma-client-js"
     previewFeatures = ["typedSql"] // ❌ Remove if present
   }
   ```

## Rollback Procedure

If you need to rollback:

### Quick Rollback

```bash
cd frontend

# Restore package.json
git checkout HEAD~1 -- package.json

# Restore lock file
mv package-lock.json.backup package-lock.json

# Reinstall
npm install
```

### Full Rollback

```bash
cd frontend

# Restore all files
git checkout HEAD~1 -- package.json package-lock.json

# Or restore from backup
cp package.json.backup package.json
cp package-lock.json.backup package-lock.json

# Reinstall
rm -rf node_modules
npm install

# Verify
npm audit  # Will show vulnerabilities again
npm test
npm run build
```

## Team Communication

### Notify Team

**Before migration:**
```
📢 Security Update Scheduled

We're updating frontend dependencies to fix 25 security vulnerabilities.

Timeline:
- Start: [DATE/TIME]
- Duration: ~15 minutes
- Impact: None (no breaking changes)

Action required:
1. Commit/push all changes
2. Pull latest after update
3. Run: npm install
4. Test your features

Questions? Ask in #dev-support
```

**After migration:**
```
✅ Security Update Complete

All 25 vulnerabilities fixed!

Next steps:
1. Pull latest changes
2. Run: cd frontend && npm install
3. Run: npm audit (should show 0 vulnerabilities)
4. Test your features
5. Report any issues in #dev-support

Documentation:
- SECURITY_UPDATE_GUIDE.md
- TESTING_CHECKLIST.md
```

## Continuous Integration

### Update CI/CD

**GitHub Actions:**

The new security workflow is already configured:
- `.github/workflows/security-scan.yml`

**Other CI systems:**

Add security scanning:

```yaml
# Example: GitLab CI
security-scan:
  script:
    - cd frontend
    - npm ci
    - npm audit --audit-level=moderate
```

## Monitoring

### Post-Migration Monitoring

**Week 1:**
- [ ] Monitor error logs daily
- [ ] Check application metrics
- [ ] Review user feedback
- [ ] Watch for performance issues

**Week 2-4:**
- [ ] Weekly security audits: `npm audit`
- [ ] Review GitHub Security Advisories
- [ ] Check Dependabot PRs
- [ ] Update documentation if needed

## Success Criteria

✅ **Migration successful if:**
- `npm audit` shows 0 vulnerabilities
- All tests pass
- Build completes successfully
- No production errors
- PDF export works correctly
- No performance degradation

## Support

### Getting Help

**Issues during migration:**
1. Check this guide
2. Review SECURITY_UPDATE_GUIDE.md
3. Check TESTING_CHECKLIST.md
4. Ask in #dev-support Slack
5. Create GitHub issue (non-security)

**Security concerns:**
- Use GitHub Security Advisories (private)
- Email: security@yourcompany.com
- Do NOT create public issues

## Additional Resources

- [SECURITY_FIX_SUMMARY.md](../SECURITY_FIX_SUMMARY.md) - Technical details
- [SECURITY_UPDATE_GUIDE.md](./SECURITY_UPDATE_GUIDE.md) - Quick reference
- [TESTING_CHECKLIST.md](./TESTING_CHECKLIST.md) - Testing guide
- [npm audit docs](https://docs.npmjs.com/cli/v10/commands/npm-audit)
- [jsPDF changelog](https://github.com/parallax/jsPDF/releases)
- [Next.js releases](https://github.com/vercel/next.js/releases)
- [Prisma releases](https://github.com/prisma/prisma/releases)

---

**Version:** 1.0  
**Last Updated:** 2026-02-24  
**Maintained By:** Development Team
