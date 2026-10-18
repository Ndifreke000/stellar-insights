/**
 * CSRF Token Management
 * 
 * Provides utilities for generating and validating CSRF tokens
 * to protect against Cross-Site Request Forgery attacks.
 *
 * Auth tokens are no longer stored in `localStorage` (see #2572). The
 * backend sets an httpOnly session cookie after SEP-10; the frontend
 * authenticates with `credentials: 'include'` and protects
 * state-changing requests with a double-submit CSRF token.
 */

/** Name of the cookie the backend sets with the CSRF token. */
export const CSRF_COOKIE_NAME = 'csrf_token'

/** Header the frontend echoes the CSRF token back in. */
export const CSRF_HEADER_NAME = 'x-csrf-token'

/**
 * Generate a cryptographically secure CSRF token
 * Uses Node.js crypto in server environment
 */
export async function generateCsrfToken(): Promise<string> {
  const array = new Uint8Array(32);
  crypto.getRandomValues(array);
  return Array.from(array, byte => byte.toString(16).padStart(2, '0')).join('');
}

/**
 * Validate CSRF token from cookie and header
 */
export function validateCsrfToken(
  cookieToken: string | undefined,
  headerToken: string | undefined
): boolean {
  if (!cookieToken || !headerToken) {
    return false;
  }

  // Constant-time comparison to prevent timing attacks
  return timingSafeEqual(cookieToken, headerToken);
}

/**
 * Read the CSRF token from the (non-httpOnly) double-submit cookie.
 * Returns undefined in non-browser environments or when absent.
 */
export function getCsrfTokenFromCookie(): string | undefined {
  if (typeof document === 'undefined') {
    return undefined;
  }

  const match = document.cookie
    .split('; ')
    .find(row => row.startsWith(`${CSRF_COOKIE_NAME}=`))

  if (!match) {
    return undefined;
  }

  return decodeURIComponent(match.slice(CSRF_COOKIE_NAME.length + 1))
}

/**
 * Build headers for a state-changing request, attaching the CSRF token
 * when available. Auth itself rides on the httpOnly cookie, so callers
 * must also pass `credentials: 'include'`.
 */
export function withCsrfHeaders(
  headers: Record<string, string> = {}
): Record<string, string> {
  const token = getCsrfTokenFromCookie()
  if (!token) {
    return headers
  }

  return { ...headers, [CSRF_HEADER_NAME]: token }
}

/**
 * Timing-safe string comparison
 */
function timingSafeEqual(a: string, b: string): boolean {
  if (a.length !== b.length) {
    return false;
  }

  let result = 0;
  for (let i = 0; i < a.length; i++) {
    result |= a.charCodeAt(i) ^ b.charCodeAt(i);
  }

  return result === 0;
}
