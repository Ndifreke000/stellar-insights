/**
 * API Client with CSRF Protection
 *
 * Provides type-safe API methods with automatic CSRF token handling
 * for all state-changing operations.
 */

import { getCsrfToken, refreshCsrfToken } from './csrf-client';

interface ApiOptions extends RequestInit {
  skipCsrf?: boolean;
}

interface ApiFetchOptions extends ApiOptions {
  _retried?: boolean;
}

/**
 * Base fetch wrapper with CSRF protection
 */
async function apiFetch(url: string, options: ApiFetchOptions = {}): Promise<Response> {
  const { skipCsrf = false, headers = {}, _retried = false, ...restOptions } = options;

  // Normalize whichever HeadersInit shape was passed in (Headers instance,
  // string[][], or a plain object) into a plain Record so it can be indexed
  // below (e.g. requestHeaders['X-CSRF-Token'] = ...).
  const requestHeaders: Record<string, string> = {
    'Content-Type': 'application/json',
    ...Object.fromEntries(new Headers(headers).entries()),
  };

  // Add CSRF token for state-changing methods
  const method = options.method?.toUpperCase();
  if (!skipCsrf && method && ['POST', 'PUT', 'DELETE', 'PATCH'].includes(method)) {
    const csrfToken = getCsrfToken();
    if (!csrfToken) {
      throw new Error('CSRF token not found. Please refresh the page.');
    }
    requestHeaders['X-CSRF-Token'] = csrfToken;
  }

  const response = await fetch(url, {
    ...restOptions,
    method,
    headers: requestHeaders,
    credentials: restOptions.credentials ?? 'same-origin',
  });

  // Re-fetch CSRF token after session expiry, then retry once
  if (response.status === 401 && !_retried) {
    await refreshCsrfToken();
    return apiFetch(url, { ...options, _retried: true });
  }

  // Handle CSRF token errors — refresh and retry once
  if (response.status === 403 && !_retried) {
    const data = await response.json().catch(() => ({}));
    if (data.error?.includes('CSRF')) {
      await refreshCsrfToken();
      return apiFetch(url, { ...options, _retried: true });
    }
    throw new Error('Security validation failed. Please refresh the page and try again.');
  }

  return response;
}

/**
 * GET request
 */
export async function apiGet<T = unknown>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, { ...options, method: 'GET' });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

/**
 * POST request with CSRF protection
 */
export async function apiPost<T = unknown>(url: string, data?: unknown, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'POST',
    body: data ? JSON.stringify(data) : undefined,
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

/**
 * PUT request with CSRF protection
 */
export async function apiPut<T = unknown>(url: string, data?: unknown, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'PUT',
    body: data ? JSON.stringify(data) : undefined,
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

/**
 * PATCH request with CSRF protection
 */
export async function apiPatch<T = unknown>(url: string, data?: unknown, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'PATCH',
    body: data ? JSON.stringify(data) : undefined,
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

/**
 * DELETE request with CSRF protection
 */
export async function apiDelete<T = unknown>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'DELETE',
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }

  return response.json();
}

export { refreshCsrfToken } from './csrf-client';
