/**
 * API Client with CSRF Protection
 *
 * Canonical fetch wrapper for the frontend. All API modules (including
 * `lib/api/api.ts` and feature `*-api.ts` modules) should delegate to the
 * helpers exported here so auth header injection and error normalization
 * stay consistent across callers.
 */

import { getCsrfToken, refreshCsrfToken } from './csrf-client';

interface ApiOptions extends RequestInit {
  skipCsrf?: boolean;
}

interface ApiFetchOptions extends ApiOptions {
  _retried?: boolean;
}

/**
 * Normalized error thrown by every API helper. Carries the HTTP status and
 * any parsed response body so callers can branch on it consistently.
 */
export class ApiError extends Error {
  readonly status: number;
  readonly statusText: string;
  readonly body: unknown;

  constructor(status: number, statusText: string, body?: unknown) {
    super(`API error: ${status} ${statusText}`);
    this.name = 'ApiError';
    this.status = status;
    this.statusText = statusText;
    this.body = body;
  }
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
 * Shared response handling: throws a normalized ApiError on non-2xx and
 * parses JSON otherwise. Used by every verb helper below.
 */
async function handleResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const body = await response.json().catch(() => undefined);
    throw new ApiError(response.status, response.statusText, body);
  }

  return response.json();
}

/**
 * GET request
 */
export async function apiGet<T = unknown>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, { ...options, method: 'GET' });
  return handleResponse<T>(response);
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
  return handleResponse<T>(response);
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
  return handleResponse<T>(response);
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
  return handleResponse<T>(response);
}

/**
 * DELETE request with CSRF protection
 */
export async function apiDelete<T = unknown>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'DELETE',
  });
  return handleResponse<T>(response);
}

export { refreshCsrfToken } from './csrf-client';
