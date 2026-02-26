# Console Logging Removal Guide

## Problem Statement

The frontend contains 50+ `console.log()`, `console.error()`, and `console.warn()` statements that expose sensitive data in the browser console in production builds.

### Data Exposed

- User account addresses
- Payment amounts and transaction hashes
- API endpoint URLs and parameters
- Error stack traces with file paths
- WebSocket subscription channels
- Authentication errors with security details

### Compliance Impact

- 🔥 **GDPR violation**: Personal data exposed in browser console
- 🔥 **Security risk**: System internals revealed to attackers
- 🔥 **PCI-DSS violation**: Transaction data visible in logs
- 🔥 **Information disclosure**: Error messages reveal architecture

## Solution Overview

1. **Logger Utility**: Created `src/lib/logger.ts` with environment-aware logging
2. **Automatic Redaction**: Sensitive data automatically redacted
3. **ESLint Rule**: Prevent future console usage
4. **Migration Script**: Automated replacement of console statements

## Logger Utility

### Features

- ✅ Environment-aware (development vs production)
- ✅ Automatic sensitive data redaction
- ✅ Structured logging with metadata
- ✅ Error tracking integration ready
- ✅ Type-safe logging methods
- ✅ Scoped loggers for components
- ✅ Performance measurement utilities

### Usage

```typescript
import { logger } from "@/lib/logger";

// Debug logging (development only)
logger.debug("User action", { action: "click", component: "Button" });

// Info logging (development only)
logger.info("Data loaded", { count: 10 });

// Warning logging (development only)
logger.warn("Deprecated feature used", { feature: "oldAPI" });

// Error logging (development + production tracking)
logger.error("API request failed", error, { endpoint: "/api/data" });

// WebSocket logging (development only)
logger.websocket("connected", { connectionId: "123" });

// API logging (development only)
logger.api("GET", "/api/corridors", { params: { limit: 10 } });

// Performance logging (development only)
logger.performance("Data fetch", 150.5, { endpoint: "/api/data" });
```

### Scoped Loggers

```typescript
import { createScopedLogger } from "@/lib/logger";

const logger = createScopedLogger("MyComponent");

logger.debug("Component mounted");
// Output: [DEBUG] [MyComponent] Component mounted
```

### Performance Measurement

```typescript
import { measurePerformance, measurePerformanceAsync } from "@/lib/logger";

// Synchronous
const result = measurePerformance("Calculate total", () => {
  return data.reduce((sum, item) => sum + item.value, 0);
});

// Asynchronous
const data = await measurePerformanceAsync("Fetch data", async () => {
  return await fetch("/api/data").then((r) => r.json());
});
```

## Automatic Redaction

The logger automatically redacts sensitive data:

### Stellar Addresses

```typescript
// Input: GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
// Output: G****[REDACTED]
```

### API Keys

```typescript
// Input: api_key_1234567890abcdef1234567890abcdef
// Output: [REDACTED_KEY]
```

### Email Addresses

```typescript
// Input: user@example.com
// Output: ****@[REDACTED]
```

### Sensitive Fields

```typescript
// Fields like password, secret, token, key, auth, credential
// are automatically redacted to [REDACTED]
```

## Migration Steps

### Step 1: Install Dependencies (if needed)

```bash
cd frontend
npm install glob --save-dev
```

### Step 2: Run Migration Script

```bash
node scripts/replace-console-statements.js
```

This script will:

- Find all files with console statements
- Replace `console.log` → `logger.debug`
- Replace `console.info` → `logger.info`
- Replace `console.warn` → `logger.warn`
- Replace `console.error` → `logger.error`
- Add logger import if missing

### Step 3: Manual Review

Review the changes for:

- Correct logger method usage
- Proper error handling
- Metadata structure
- Sensitive data handling

### Step 4: Update ESLint Configuration

The ESLint rule is already configured in `.eslintrc.json` to prevent future console usage.

### Step 5: Test

```bash
# Run tests
npm test

# Run linter
npm run lint

# Build production bundle
npm run build

# Verify no console statements in production
grep -r "console\." .next/
# Expected: No results
```

## Files to Update

### High Priority (Security Critical)

1. **`src/services/sep10Auth.ts`** ✅ UPDATED
   - Wallet signing errors
   - Challenge validation errors
   - Authentication flow logging

2. **`src/lib/websocket.ts`** - 11 console statements
   - Connection status
   - Error handling
   - Message parsing

3. **`src/hooks/useWebSocket.ts`** - 5 console statements
   - Connection events
   - Message handling
   - Error logging

4. **`src/lib/api.ts`** - 3 console statements
   - API request errors
   - Network errors

### Medium Priority (Data Exposure)

5. **`src/app/api/dashboard/route.ts`** - 1 console statement
6. **`src/app/[locale]/dashboard/page.tsx`** - 3 console statements
7. **`src/app/[locale]/corridors/[pair]/page.tsx`** - 4 console statements
8. **`src/hooks/useRealtimeCorridors.ts`** - 3 console statements
9. **`src/lib/trustline-api.ts`** - 3 console statements
10. **`src/lib/liquidity-pool-api.ts`** - 1 console statement
11. **`src/lib/analytics-api.ts`** - 2 console statements

### Low Priority (Development/Debug)

12. **`src/lib/monitoring.ts`** - 3 console statements
13. **`src/lib/keyboard-shortcuts/registry.ts`** - 1 console statement
14. **`src/app/[locale]/trustlines/page.tsx`** - 1 console statement
15. **`src/app/[locale]/transactions/builder/page.tsx`** - 1 console statement
16. **`src/app/[locale]/corridors/page.tsx`** - 1 console statement
17. **`src/app/[locale]/corridors/compare/page.tsx`** - 2 console statements
18. **`src/app/[locale]/anchors/[address]/page.tsx`** - 1 console statement
19. **`src/app/[locale]/anchors/page.tsx`** - 1 console statement
20. **`src/app/[locale]/analytics/page.tsx`** - 2 console statements
21. **`src/app/alerts/page.tsx`** - 7 console statements
22. **`src/app/api/network-graph/route.ts`** - 1 console statement
23. **`src/app/api-docs/page.tsx`** - 1 console statement (in code example)
24. **`src/app/api-docs/examples/page.tsx`** - 5 console statements (in examples)

## ESLint Configuration

### Current Rule

```json
{
  "rules": {
    "no-console": [
      "error",
      {
        "allow": []
      }
    ]
  }
}
```

This prevents ALL console methods in the codebase.

### Exceptions

For code examples in documentation (`api-docs`), you can add:

```json
{
  "overrides": [
    {
      "files": ["src/app/api-docs/**/*.tsx"],
      "rules": {
        "no-console": "off"
      }
    }
  ]
}
```

## Testing

### Unit Tests

```typescript
import { logger, __testing__ } from "@/lib/logger";

describe("Logger", () => {
  it("should redact Stellar addresses", () => {
    const input =
      "Account: GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX";
    const output = __testing__.redactSensitiveData(input);
    expect(output).toContain("G****[REDACTED]");
  });

  it("should redact sensitive fields", () => {
    const input = { password: "secret123", username: "user" };
    const output = __testing__.redactSensitiveData(input);
    expect(output).toEqual({ password: "[REDACTED]", username: "user" });
  });
});
```

### Integration Tests

```typescript
describe("Production Build", () => {
  it("should not contain console statements", () => {
    // Check production bundle
    const buildFiles = glob.sync(".next/**/*.js");

    buildFiles.forEach((file) => {
      const content = fs.readFileSync(file, "utf8");
      expect(content).not.toMatch(/console\.(log|error|warn|info|debug)/);
    });
  });
});
```

## Production Verification

### Before Deployment

```bash
# Build production bundle
npm run build

# Check for console statements
find .next -name "*.js" -exec grep -l "console\." {} \;

# Expected: No results (or only in vendor bundles)
```

### After Deployment

1. Open browser DevTools
2. Navigate through the application
3. Check Console tab
4. Expected: No application logs in production

## Error Tracking Integration

### Sentry Integration (Example)

```typescript
// src/lib/logger.ts

import * as Sentry from "@sentry/nextjs";

function sendToErrorTracking(error: Error, metadata?: LogMetadata): void {
  if (isDevelopment || !isLoggingEnabled) {
    return;
  }

  Sentry.captureException(error, {
    extra: redactSensitiveData(metadata),
    tags: {
      environment: process.env.NODE_ENV,
      component: metadata?.component as string,
    },
  });
}
```

### LogRocket Integration (Example)

```typescript
import LogRocket from "logrocket";

function sendToErrorTracking(error: Error, metadata?: LogMetadata): void {
  if (isDevelopment || !isLoggingEnabled) {
    return;
  }

  LogRocket.captureException(error, {
    extra: redactSensitiveData(metadata),
  });
}
```

## Best Practices

### DO ✅

```typescript
// Use logger with structured metadata
logger.debug("User action", { action: "click", component: "Button" });

// Use error logging with error object
logger.error("API request failed", error, { endpoint: "/api/data" });

// Use scoped loggers for components
const logger = createScopedLogger("MyComponent");

// Use performance measurement
const result = await measurePerformanceAsync("Fetch data", fetchData);
```

### DON'T ❌

```typescript
// Don't use console directly
console.log("User action");

// Don't log sensitive data without redaction
logger.debug("User password", { password: userPassword });

// Don't use string interpolation with sensitive data
logger.debug(`User ${userId} logged in`);

// Don't log in production without purpose
if (process.env.NODE_ENV === "production") {
  logger.debug("Debug info"); // This won't log anyway
}
```

## Rollback Plan

If issues arise after migration:

1. **Revert changes**:

   ```bash
   git revert <commit-hash>
   ```

2. **Temporary fix** (not recommended):

   ```typescript
   // src/lib/logger.ts
   const isDevelopment = true; // Force development mode
   ```

3. **Gradual rollout**:
   - Update high-priority files first
   - Test thoroughly
   - Update remaining files

## Success Metrics

- ✅ Zero console statements in production bundle
- ✅ All sensitive data redacted in logs
- ✅ ESLint passing with no-console rule
- ✅ No security warnings in browser console
- ✅ Error tracking integration working
- ✅ Performance impact < 1ms per log statement

## Support

For questions or issues:

- Check this guide first
- Review `src/lib/logger.ts` implementation
- Contact security team for sensitive data concerns
- Open issue with `[logging]` tag

---

**Status**: Implementation in progress
**Priority**: HIGH (Security & Compliance)
**Target**: Complete before next production deployment
