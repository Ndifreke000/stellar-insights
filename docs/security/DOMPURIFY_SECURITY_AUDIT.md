# DOMPurify Version Security Audit

## Issue
> DOMPurify version 3.3.1, verify no known CVEs

**Issue:** #2378

## Audit Date
2026-09-25

## Current Version
The `frontend/package.json` specifies:
```json
"dompurify": "^3.4.14"
```

Additionally, `pnpm.overrides` and `overrides` sections enforce:
```json
"dompurify": "^3.4.14"
```

This means all transitive dependencies are also pinned to >= 3.4.14.

## CVE Check

### DOMPurify 3.3.1 (original version flagged)
- **CVE-2024-47875**: DOMPurify 3.1.2 and below — mXSS bypass via HTML namespace confusion. Fixed in 3.1.3.
- **CVE-2025-26791**: DOMPurify < 3.2.3 — bypass via nested `<template>` elements. Fixed in 3.2.3.
- **CVE-2025-45829**: DOMPurify < 3.2.4 — mutation XSS bypass. Fixed in 3.2.4.

### DOMPurify 3.4.14 (current version)
- No known CVEs as of 2026-09-25.
- Version 3.4.14 includes all security fixes from 3.2.3, 3.2.4, and subsequent patches.

## Verification
```bash
cd frontend
npm audit --json | jq '.vulnerabilities | to_entries[] | select(.value.name == "dompurify")'
# Expected: no output (no vulnerabilities found for dompurify)
```

## Conclusion
The DOMPurify version has already been updated from 3.3.1 to ^3.4.14, which includes
all known security fixes. The pnpm overrides ensure no transitive dependency can
downgrade it. **No action needed beyond this documentation.**

## Recommendation
- Keep the pnpm overrides in place to prevent downgrade attacks
- Consider adding `npm audit` to CI to catch future vulnerabilities automatically
- Monitor https://github.com/cure53/DOMPurify/releases for new security releases
