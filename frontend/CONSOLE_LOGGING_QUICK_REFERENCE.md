# Console Logging - Quick Reference

## Quick Migration

```bash
# Run automated migration
cd frontend
node scripts/replace-console-statements.js

# Review changes
git diff

# Test
npm run lint
npm test

# Commit
git add .
git commit -m "feat: replace console statements with logger"
```

## Logger Import

```typescript
import { logger } from "@/lib/logger";
```

## Common Replacements

| Before               | After               |
| -------------------- | ------------------- |
| `console.log(...)`   | `logger.debug(...)` |
| `console.info(...)`  | `logger.info(...)`  |
| `console.warn(...)`  | `logger.warn(...)`  |
| `console.error(...)` | `logger.error(...)` |

## Usage Examples

### Basic Logging

```typescript
// Debug (development only)
logger.debug("User clicked button");

// Info (development only)
logger.info("Data loaded successfully");

// Warning (development only)
logger.warn("Deprecated API used");

// Error (development + production tracking)
logger.error("API request failed", error);
```

### With Metadata

```typescript
logger.debug("User action", {
  action: "click",
  component: "Button",
  timestamp: Date.now(),
});

logger.error("Request failed", error, {
  endpoint: "/api/data",
  method: "GET",
  statusCode: 500,
});
```

### WebSocket Logging

```typescript
logger.websocket("connected", { connectionId: "123" });
logger.websocket("message_received", { type: "corridor_update" });
logger.websocket("disconnected", { reason: "timeout" });
```

### API Logging

```typescript
logger.api("GET", "/api/corridors", { params: { limit: 10 } });
logger.api("POST", "/api/auth", { success: true });
```

### Performance Logging

```typescript
// Synchronous
const result = measurePerformance("Calculate total", () => {
  return data.reduce((sum, item) => sum + item.value, 0);
});

// Asynchronous
const data = await measurePerformanceAsync("Fetch data", async () => {
  return await fetchData();
});
```

### Scoped Logger

```typescript
import { createScopedLogger } from "@/lib/logger";

const logger = createScopedLogger("MyComponent");

logger.debug("Component mounted");
// Output: [DEBUG] [MyComponent] Component mounted
```

## Automatic Redaction

The logger automatically redacts:

- **Stellar addresses**: `GXXX...` → `G****[REDACTED]`
- **API keys**: `sk_live_...` → `[REDACTED_KEY]`
- **Emails**: `user@example.com` → `****@[REDACTED]`
- **Sensitive fields**: `password`, `secret`, `token` → `[REDACTED]`

## Environment Behavior

| Method          | Development        | Production           |
| --------------- | ------------------ | -------------------- |
| `debug()`       | ✅ Logs to console | ❌ Silent            |
| `info()`        | ✅ Logs to console | ❌ Silent            |
| `warn()`        | ✅ Logs to console | ❌ Silent            |
| `error()`       | ✅ Logs to console | ✅ Sends to tracking |
| `websocket()`   | ✅ Logs to console | ❌ Silent            |
| `api()`         | ✅ Logs to console | ❌ Silent            |
| `performance()` | ✅ Logs to console | ❌ Silent            |

## ESLint Rule

```json
{
  "rules": {
    "no-console": ["error", { "allow": [] }]
  }
}
```

This prevents ALL console usage. Use logger instead.

## Testing

```bash
# Check for console statements
grep -r "console\." src/

# Run linter
npm run lint

# Run tests
npm test

# Build and verify
npm run build
find .next -name "*.js" -exec grep -l "console\." {} \;
```

## Common Patterns

### Error Handling

```typescript
try {
  await fetchData();
} catch (error) {
  logger.error("Failed to fetch data", error, {
    endpoint: "/api/data",
    retry: retryCount,
  });
}
```

### Conditional Logging

```typescript
// ❌ DON'T - Logger handles this automatically
if (process.env.NODE_ENV === "development") {
  logger.debug("Debug info");
}

// ✅ DO - Just call logger
logger.debug("Debug info");
```

### WebSocket Events

```typescript
ws.onopen = () => {
  logger.websocket("connected");
};

ws.onerror = (error) => {
  logger.error("WebSocket error", error);
};

ws.onclose = () => {
  logger.websocket("disconnected");
};
```

### API Requests

```typescript
async function fetchData() {
  logger.api("GET", "/api/data");

  try {
    const response = await fetch("/api/data");
    const data = await response.json();

    logger.debug("Data fetched", { count: data.length });
    return data;
  } catch (error) {
    logger.error("Fetch failed", error, { endpoint: "/api/data" });
    throw error;
  }
}
```

## DO ✅

- Use logger for all logging
- Include metadata for context
- Use appropriate log levels
- Let logger handle redaction
- Use scoped loggers for components

## DON'T ❌

- Don't use console directly
- Don't log sensitive data manually
- Don't use string interpolation with sensitive data
- Don't log excessively in loops
- Don't ignore ESLint warnings

## Files to Check

High priority files with console statements:

- `src/lib/websocket.ts` (11 statements)
- `src/hooks/useWebSocket.ts` (5 statements)
- `src/services/sep10Auth.ts` (11 statements) ✅ DONE
- `src/lib/api.ts` (3 statements)
- `src/app/api/dashboard/route.ts` (1 statement)

## Quick Commands

```bash
# Find all console statements
rg "console\.(log|error|warn|info|debug)" src/

# Count console statements
rg "console\.(log|error|warn)" src/ | wc -l

# Run migration
node scripts/replace-console-statements.js

# Verify no console in production build
npm run build && grep -r "console\." .next/
```

## Support

- **Full Guide**: `CONSOLE_LOGGING_REMOVAL_GUIDE.md`
- **Resolution**: `CONSOLE_LOGGING_RESOLUTION.md`
- **Logger Source**: `src/lib/logger.ts`
- **Migration Script**: `scripts/replace-console-statements.js`

---

**Remember**: Logger is environment-aware. Just use it and it handles dev vs prod automatically!
