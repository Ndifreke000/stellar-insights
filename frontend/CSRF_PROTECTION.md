# CSRF Protection Implementation

## Overview

This document describes the CSRF (Cross-Site Request Forgery) protection implementation for Stellar Insights frontend API routes.

## Security Issue Addressed

**Issue #467: Missing CSRF Protection**
- Severity: 🔴 Critical (Security)
- Location: Frontend API routes
- Status: ✅ Resolved

## Implementation Details

### 1. Core CSRF Module (`src/lib/csrf.ts`)

Provides cryptographically secure token generation and validation:

- `generateCsrfToken()`: Generates 32-byte random tokens using Node.js crypto
- `validateCsrfToken()`: Performs timing-safe token comparison to prevent timing attacks

### 2. Middleware Protection (`src/middleware.ts`)

Implements automatic CSRF protection for all API routes:

**For GET requests:**
- Generates a new CSRF token
- Sets token in httpOnly cookie (`csrf-token`)
- Includes token in response header (`X-CSRF-Token`)

**For state-changing requests (POST, PUT, DELETE, PATCH):**
- Validates token from cookie matches token from header
- Returns 403 Forbidden if validation fails
- Allows request to proceed if validation succeeds

**Cookie Configuration:**
- `httpOnly: true` - Prevents JavaScript access
- `secure: true` (production) - HTTPS only
- `sameSite: 'strict'` - Prevents cross-site requests
- `maxAge: 86400` - 24-hour expiration

### 3. API Client (`src/lib/api-client.ts`)

Type-safe API methods with automatic CSRF token handling:

- `apiGet()` - GET requests (no CSRF token needed)
- `apiPost()` - POST requests with CSRF protection
- `apiPut()` - PUT requests with CSRF protection
- `apiPatch()` - PATCH requests with CSRF protection
- `apiDelete()` - DELETE requests with CSRF protection

**Features:**
- Automatic token extraction from cookie or meta tag
- Clear error messages for missing tokens
- Handles 403 CSRF validation errors gracefully

### 4. React Component (`src/components/CsrfTokenProvider.tsx`)

Client-side component that injects CSRF token as meta tag for easy access.

## Usage Examples

### Backend API Route (Future Implementation)

```typescript
// src/app/api/dashboard/route.ts
import { NextResponse } from 'next/server';

export async function POST(request: Request) {
  // CSRF validation happens automatically in middleware
  const data = await request.json();
  
  // Process the request
  return NextResponse.json({ success: true });
}
```

### Frontend Client Code

```typescript
import { apiPost, apiDelete } from '@/lib/api-client';

// Example: Create a new resource
async function createResource(data: any) {
  try {
    const result = await apiPost('/api/dashboard', data);
    console.log('Success:', result);
  } catch (error) {
    console.error('Failed:', error);
  }
}

// Example: Delete a resource
async function deleteResource(id: string) {
  try {
    await apiDelete(`/api/dashboard/${id}`);
    console.log('Deleted successfully');
  } catch (error) {
    console.error('Failed:', error);
  }
}
```

### Manual Fetch (Alternative)

```typescript
async function manualApiCall() {
  const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
  
  const response = await fetch('/api/dashboard', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'X-CSRF-Token': csrfToken || ''
    },
    body: JSON.stringify({ action: 'create' })
  });
  
  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }
  
  return response.json();
}
```

## Testing

### Unit Tests

Run the test suite:

```bash
cd frontend
npm test src/__tests__/csrf.test.ts
npm test src/__tests__/api-client.test.ts
```

### Manual Testing

#### Test 1: Missing CSRF Token (Should Fail)

```bash
curl -X POST http://localhost:3000/api/dashboard \
  -H "Content-Type: application/json" \
  -d '{"action":"delete"}'
```

Expected: `403 Forbidden` with error message

#### Test 2: Invalid CSRF Token (Should Fail)

```bash
curl -X POST http://localhost:3000/api/dashboard \
  -H "Content-Type: application/json" \
  -H "X-CSRF-Token: invalid-token" \
  -H "Cookie: csrf-token=different-token" \
  -d '{"action":"delete"}'
```

Expected: `403 Forbidden` with error message

#### Test 3: Valid CSRF Token (Should Succeed)

```bash
# First, get the token
TOKEN=$(curl -s http://localhost:3000/api/dashboard | grep -o 'X-CSRF-Token: [^"]*' | cut -d' ' -f2)

# Then use it
curl -X POST http://localhost:3000/api/dashboard \
  -H "Content-Type: application/json" \
  -H "X-CSRF-Token: $TOKEN" \
  -H "Cookie: csrf-token=$TOKEN" \
  -d '{"action":"delete"}'
```

Expected: `200 OK` (or appropriate response)

## Security Benefits

✅ **Prevents CSRF Attacks**: Validates that requests originate from legitimate sources
✅ **Timing Attack Protection**: Uses constant-time comparison for token validation
✅ **Secure Cookie Configuration**: httpOnly, secure, sameSite flags prevent token theft
✅ **Automatic Protection**: Middleware applies to all API routes without manual intervention
✅ **Developer-Friendly**: Simple API client abstracts complexity

## Attack Scenarios Mitigated

### Before Implementation

1. User logs into Stellar Insights
2. User visits malicious website
3. Malicious site makes POST request to Stellar Insights API
4. Request includes user's cookies (automatic)
5. ❌ API processes request as legitimate user
6. ❌ Attacker performs unauthorized actions

### After Implementation

1. User logs into Stellar Insights
2. User visits malicious website
3. Malicious site makes POST request to Stellar Insights API
4. Request includes user's cookies but NO CSRF token
5. ✅ Middleware validates CSRF token
6. ✅ Request rejected with 403 Forbidden
7. ✅ Attack prevented

## Migration Guide

### For Existing API Routes

No changes needed! CSRF protection is automatically applied by middleware.

### For New API Routes

Use the provided API client:

```typescript
import { apiPost } from '@/lib/api-client';

// Instead of:
// fetch('/api/endpoint', { method: 'POST', ... })

// Use:
apiPost('/api/endpoint', data);
```

### For Custom Fetch Calls

Include the CSRF token header:

```typescript
const csrfToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');

fetch('/api/endpoint', {
  method: 'POST',
  headers: {
    'X-CSRF-Token': csrfToken || ''
  }
});
```

## Configuration

### Environment Variables

- `NODE_ENV=production` - Enables secure cookie flag (HTTPS only)

### Middleware Configuration

Located in `src/middleware.ts`:

```typescript
export const config = {
  matcher: [
    "/((?!_next|_vercel|.*\\..*).*)", // All routes except static files
  ],
};
```

## Troubleshooting

### "CSRF token not found" Error

**Cause**: Token cookie not set or expired

**Solution**: 
1. Ensure user has made a GET request first to receive token
2. Check cookie expiration (24 hours)
3. Verify cookie is not blocked by browser settings

### "Invalid CSRF token" Error

**Cause**: Token mismatch between cookie and header

**Solution**:
1. Refresh the page to get a new token
2. Verify token is being sent in `X-CSRF-Token` header
3. Check for token tampering or corruption

### Token Not Appearing in Meta Tag

**Cause**: CsrfTokenProvider not included in layout

**Solution**: Add to root layout:

```typescript
import { CsrfTokenProvider } from '@/components/CsrfTokenProvider';

export default function RootLayout({ children }) {
  return (
    <html>
      <head>
        <CsrfTokenProvider />
      </head>
      <body>{children}</body>
    </html>
  );
}
```

## References

- [OWASP CSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- [Next.js Middleware Documentation](https://nextjs.org/docs/app/building-your-application/routing/middleware)
- [MDN: CSRF](https://developer.mozilla.org/en-US/docs/Glossary/CSRF)

## Status

✅ **Implementation Complete**
- Core CSRF module implemented
- Middleware protection active
- API client with automatic token handling
- Unit tests passing
- Documentation complete

🔒 **Security Level: High**
- Cryptographically secure tokens
- Timing attack protection
- Secure cookie configuration
- Automatic enforcement
