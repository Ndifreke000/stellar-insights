/**
 * Client-side CSRF token utilities.
 *
 * The server sets an httpOnly cookie; the readable copy lives in a meta tag
 * and the X-CSRF-Token response header on GET /api/* requests.
 */

export const CSRF_TOKEN_REFRESHED_EVENT = 'csrf-token-refreshed';
export const CSRF_REFRESH_PATH = '/api/example';

export function getCsrfTokenFromMeta(): string | null {
  return document.querySelector('meta[name="csrf-token"]')?.getAttribute('content') ?? null;
}

export function getCsrfTokenFromCookie(): string | null {
  const cookies = document.cookie.split(';');
  for (const cookie of cookies) {
    const [name, value] = cookie.trim().split('=');
    if (name === 'csrf-token') {
      return decodeURIComponent(value);
    }
  }
  return null;
}

export function getCsrfToken(): string | null {
  return getCsrfTokenFromMeta() ?? getCsrfTokenFromCookie();
}

export function updateCsrfTokenMeta(token: string): void {
  let meta = document.querySelector('meta[name="csrf-token"]');
  if (!meta) {
    meta = document.createElement('meta');
    meta.setAttribute('name', 'csrf-token');
    document.head.appendChild(meta);
  }
  meta.setAttribute('content', token);
  window.dispatchEvent(new CustomEvent(CSRF_TOKEN_REFRESHED_EVENT, { detail: { token } }));
}

/** Fetch a fresh CSRF token from the server via a lightweight GET request. */
export async function refreshCsrfToken(): Promise<string> {
  const response = await fetch(CSRF_REFRESH_PATH, {
    method: 'GET',
    credentials: 'same-origin',
  });

  if (!response.ok) {
    throw new Error('Failed to refresh CSRF token');
  }

  const token = response.headers.get('X-CSRF-Token');
  if (!token) {
    throw new Error('CSRF token missing from refresh response');
  }

  updateCsrfTokenMeta(token);
  return token;
}
