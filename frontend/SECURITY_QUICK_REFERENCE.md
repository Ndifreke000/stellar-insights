# Security Quick Reference Guide

## Quick Commands

### Check for Vulnerabilities
```bash
cd frontend
npm audit
```

### Fix Vulnerabilities Automatically
```bash
# Non-breaking fixes
npm audit fix

# Breaking fixes (requires testing)
npm audit fix --force
```

### Update Specific Package
```bash
npm update <package-name>
npm install <package-name>@latest
```

### Use Security Update Script
```bash
# Windows
./scripts/security-update.ps1

# Unix/Linux/macOS
chmod +x ./scripts/security-update.sh
./scripts/security-update.sh
```

## Package Versions

| Package | Secure Version | Notes |
|---------|---------------|-------|
| jspdf | ^2.5.2 | Fixed 7 high severity vulnerabilities |
| jspdf-autotable | ^3.8.3 | Compatible with jspdf 2.5.2 |
| hono | N/A | Not used in this project |
| lodash | N/A | Use native JavaScript instead |

## Native JavaScript Alternatives

### Instead of Lodash

```javascript
// _.omit(obj, ['key'])
const { key, ...rest } = obj;

// _.pick(obj, ['key1', 'key2'])
const picked = (({ key1, key2 }) => ({ key1, key2 }))(obj);

// _.groupBy(items, 'category')
const grouped = items.reduce((acc, item) => {
  (acc[item.category] = acc[item.category] || []).push(item);
  return acc;
}, {});

// _.uniq(array)
const unique = [...new Set(array)];

// _.chunk(array, size)
const chunked = Array.from(
  { length: Math.ceil(arr.length / size) },
  (_, i) => arr.slice(i * size, i * size + size)
);

// _.debounce(fn, wait)
const debounce = (fn, wait) => {
  let timeout;
  return (...args) => {
    clearTimeout(timeout);
    timeout = setTimeout(() => fn(...args), wait);
  };
};

// _.throttle(fn, wait)
const throttle = (fn, wait) => {
  let lastCall = 0;
  return (...args) => {
    const now = Date.now();
    if (now - lastCall >= wait) {
      lastCall = now;
      fn(...args);
    }
  };
};
```

## CI/CD Integration

### GitHub Actions Workflow
Location: `.github/workflows/security.yml`

Runs on:
- Push to main/develop
- Pull requests
- Weekly schedule (Sunday 00:00 UTC)

### Manual Trigger
```bash
# Via GitHub UI
Actions → Security Scan → Run workflow
```

## Security Checklist

### Before Deployment
- [ ] Run `npm audit` - 0 vulnerabilities
- [ ] Run `npm test` - all tests pass
- [ ] Test PDF export functionality
- [ ] Test CSV export functionality
- [ ] Test JSON export functionality
- [ ] Review dependency changes
- [ ] Update CHANGELOG.md

### Monthly Maintenance
- [ ] Check for outdated packages: `npm outdated`
- [ ] Review security advisories
- [ ] Update non-breaking dependencies
- [ ] Test application thoroughly
- [ ] Update documentation

### Quarterly Review
- [ ] Major version updates
- [ ] Security audit of all dependencies
- [ ] Review and update security policies
- [ ] Penetration testing (if applicable)

## Troubleshooting

### Issue: npm audit shows vulnerabilities
```bash
# Try automatic fix
npm audit fix

# If that doesn't work
npm audit fix --force

# Check what will be updated
npm audit fix --dry-run
```

### Issue: Breaking changes after update
```bash
# Restore from backup
cp package-lock.json.backup package-lock.json
npm install

# Or revert via git
git checkout HEAD~1 -- package.json package-lock.json
npm install
```

### Issue: Peer dependency conflicts
```bash
# Install with legacy peer deps
npm install --legacy-peer-deps

# Or force install
npm install --force
```

## Resources

- **npm audit docs:** https://docs.npmjs.com/cli/v8/commands/npm-audit
- **Security advisories:** https://github.com/advisories
- **Snyk vulnerability DB:** https://snyk.io/vuln/
- **OWASP Top 10:** https://owasp.org/www-project-top-ten/

## Emergency Contacts

For critical security issues:
1. Create private security advisory on GitHub
2. Email: security@stellar-insights.example.com
3. Report via GitHub Issues (for non-critical)

## Status Badges

Add to README.md:
```markdown
![Security Scan](https://github.com/Ndifreke000/stellar-insights/workflows/Security%20Scan/badge.svg)
```

---

**Quick Tip:** Run `npm audit` before every deployment!
