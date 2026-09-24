# Frontend Error Tracking with Sentry

## Overview

PayRaider uses Sentry for comprehensive frontend error tracking, including:
- Real-time error notifications
- Source map support for readable stack traces
- User context and breadcrumbs for debugging
- Release tracking for version correlation
- Error sampling and filtering

## Setup

### 1. Environment Variables

Create `.env.local` with Sentry configuration:

```bash
# Sentry DSN for frontend error tracking
NEXT_PUBLIC_SENTRY_DSN=https://your-key@sentry.io/your-project-id

# Application version for release tracking
NEXT_PUBLIC_APP_VERSION=1.0.0

# Sentry CLI configuration for source map uploads
SENTRY_AUTH_TOKEN=your-auth-token
SENTRY_ORG=your-org
SENTRY_PROJECT=your-project
```

### 2. Install Sentry CLI

```bash
npm install --save-dev @sentry/cli
# or
brew install getsentry/tools/sentry-cli
```

## Features

### Source Map Upload

Source maps are automatically uploaded during the build process:

```bash
npm run build
# Automatically uploads source maps to Sentry
```

Or manually:

```bash
./frontend/scripts/upload-sourcemaps.sh
```

### User Context

Errors are automatically tagged with user information:

```typescript
import * as Sentry from "@sentry/nextjs";

// Set user context
Sentry.setUser({
  id: userId,
  email: userEmail,
  username: userName,
});

// Clear user context on logout
Sentry.setUser(null);
```

### Breadcrumbs

Breadcrumbs track user actions leading up to an error:

```typescript
import * as Sentry from "@sentry/nextjs";

// Automatic breadcrumbs for:
// - Console logs
// - DOM interactions
// - Network requests
// - Page navigation
// - XHR requests

// Manual breadcrumb
Sentry.addBreadcrumb({
  category: 'user-action',
  message: 'User clicked button',
  level: 'info',
  data: { buttonId: 'submit-btn' },
});
```

### Release Tracking

Errors are correlated with application releases:

```typescript
// Release is set from NEXT_PUBLIC_APP_VERSION
// Allows filtering errors by version in Sentry dashboard
```

### Error Sampling

Configure error sampling rate:

```javascript
// In sentry.client.config.js
Sentry.init({
  errorSampleRate: 1.0, // 100% for now
  // Adjust to 0.1 for 10% sampling in production
});
```

## Usage

### Logging Errors

```typescript
import { logger } from '@/lib/logger';

try {
  // Some operation
} catch (error) {
  logger.error('Operation failed', error, {
    operation: 'fetchData',
    endpoint: '/api/data',
  });
}
```

### Capturing Exceptions

```typescript
import * as Sentry from "@sentry/nextjs";

try {
  // Code that might fail
} catch (error) {
  Sentry.captureException(error, {
    tags: {
      section: 'payment',
      action: 'process',
    },
    extra: {
      amount: 100,
      currency: 'USD',
    },
  });
}
```

### Capturing Messages

```typescript
import * as Sentry from "@sentry/nextjs";

Sentry.captureMessage('User action completed', 'info', {
  tags: {
    action: 'export',
  },
});
```

## Sentry Dashboard

### Viewing Errors

1. Go to [sentry.io](https://sentry.io)
2. Select your organization and project
3. View errors in the Issues tab

### Filtering

- **By Release**: Filter errors by application version
- **By User**: Find all errors for a specific user
- **By Tag**: Filter by custom tags (section, action, etc.)
- **By Environment**: Separate development, staging, production

### Stack Traces

- Click on an issue to view full stack trace
- Source maps enable readable file names and line numbers
- View breadcrumbs leading up to the error

## Configuration

### Ignore Errors

Certain errors are automatically ignored:

```javascript
ignoreErrors: [
  'chrome-extension://',
  'moz-extension://',
  'NetworkError',
  'Network request failed',
],
```

### Before Send Hook

Customize what gets sent to Sentry:

```javascript
beforeSend(event, hint) {
  // Filter out errors from browser extensions
  if (event.exception) {
    const error = hint.originalException;
    if (error && typeof error === 'string' && error.includes('chrome-extension')) {
      return null;
    }
  }
  return event;
}
```

## Best Practices

### 1. Use Meaningful Tags

```typescript
Sentry.captureException(error, {
  tags: {
    feature: 'payment-processing',
    severity: 'critical',
    user_action: 'checkout',
  },
});
```

### 2. Attach Context

```typescript
Sentry.captureException(error, {
  extra: {
    userId: user.id,
    orderId: order.id,
    amount: order.total,
  },
});
```

### 3. Use Breadcrumbs

```typescript
Sentry.addBreadcrumb({
  category: 'api-call',
  message: 'Fetching user data',
  level: 'info',
  data: { endpoint: '/api/users/123' },
});
```

### 4. Set User Context

```typescript
Sentry.setUser({
  id: userId,
  email: userEmail,
  username: userName,
});
```

### 5. Redact Sensitive Data

The logger automatically redacts:
- Stellar addresses (G...)
- API keys
- Email addresses
- Passwords and tokens

## Troubleshooting

### Source Maps Not Working

1. Verify `SENTRY_AUTH_TOKEN` is set
2. Check release version matches `NEXT_PUBLIC_APP_VERSION`
3. Run: `./frontend/scripts/upload-sourcemaps.sh`
4. Verify in Sentry dashboard: Settings → Releases

### Errors Not Appearing

1. Check `NEXT_PUBLIC_SENTRY_DSN` is correct
2. Verify environment is not development (errors only sent in production)
3. Check browser console for Sentry errors
4. Verify error sampling rate is > 0

### Too Many Errors

1. Adjust `errorSampleRate` to sample errors
2. Add more filters in `ignoreErrors`
3. Use `beforeSend` to filter specific errors

## References

- [Sentry Documentation](https://docs.sentry.io/)
- [Sentry Next.js Integration](https://docs.sentry.io/platforms/javascript/guides/nextjs/)
- [Source Maps Guide](https://docs.sentry.io/platforms/javascript/sourcemaps/)
- [Breadcrumbs Documentation](https://docs.sentry.io/platforms/javascript/enriching-events/breadcrumbs/)
