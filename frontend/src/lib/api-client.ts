/**
 * API Client with CSRF Protection
 * 
 * Provides type-safe API methods with automatic CSRF token handling
 * for all state-changing operations.
 */

interface ApiOptions extends RequestInit {
  skipCsrf?: boolean;
}

/**
 * Get CSRF token from meta tag or cookie
 */
function getCsrfToken(): string | null {
  // Try meta tag first (for SSR pages)
  const metaToken = document.querySelector('meta[name="csrf-token"]')?.getAttribute('content');
  if (metaToken) return metaToken;
  
  // Fallback to cookie
  const cookies = document.cookie.split(';');
  for (const cookie of cookies) {
    const [name, value] = cookie.trim().split('=');
    if (name === 'csrf-token') {
      return decodeURIComponent(value);
    }
  }
  
  return null;
}

/**
 * Base fetch wrapper with CSRF protection
 */
async function apiFetch(url: string, options: ApiOptions = {}): Promise<Response> {
  const { skipCsrf = false, headers = {}, ...restOptions } = options;
  
  const requestHeaders: HeadersInit = {
    'Content-Type': 'application/json',
    ...headers,
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
  });
  
  // Handle CSRF token errors
  if (response.status === 403) {
    const data = await response.json().catch(() => ({}));
    if (data.error?.includes('CSRF')) {
      throw new Error('Security validation failed. Please refresh the page and try again.');
    }
  }
  
  return response;
}

/**
 * GET request
 */
export async function apiGet<T = any>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, { ...options, method: 'GET' });
  
  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }
  
  return response.json();
}

/**
 * POST request with CSRF protection
 */
export async function apiPost<T = any>(url: string, data?: any, options?: ApiOptions): Promise<T> {
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
export async function apiPut<T = any>(url: string, data?: any, options?: ApiOptions): Promise<T> {
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
export async function apiPatch<T = any>(url: string, data?: any, options?: ApiOptions): Promise<T> {
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
export async function apiDelete<T = any>(url: string, options?: ApiOptions): Promise<T> {
  const response = await apiFetch(url, {
    ...options,
    method: 'DELETE',
  });
  
  if (!response.ok) {
    throw new Error(`API error: ${response.status} ${response.statusText}`);
  }
  
  return response.json();
}
