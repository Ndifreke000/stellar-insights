/**
 * CSRF Token Management
 * 
 * Provides utilities for generating and validating CSRF tokens
 * to protect against Cross-Site Request Forgery attacks.
 */

/**
 * Generate a cryptographically secure CSRF token
 * Uses Node.js crypto in server environment
 */
export async function generateCsrfToken(): Promise<string> {
  // Dynamic import for server-side only
  const { randomBytes } = await import('crypto');
  return randomBytes(32).toString('hex');
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
