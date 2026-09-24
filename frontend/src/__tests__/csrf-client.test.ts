/**
 * CSRF client utility tests
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  CSRF_TOKEN_REFRESHED_EVENT,
  CSRF_REFRESH_PATH,
  getCsrfToken,
  refreshCsrfToken,
  updateCsrfTokenMeta,
} from '../lib/csrf-client';

global.fetch = vi.fn();
const mockFetch = vi.mocked(global.fetch);

describe('csrf-client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    document.head.innerHTML = '';
    Object.defineProperty(document, 'cookie', {
      writable: true,
      value: 'csrf-token=cookie-token',
    });
  });

  it('updateCsrfTokenMeta sets meta tag content', () => {
    updateCsrfTokenMeta('meta-token-123');
    const meta = document.querySelector('meta[name="csrf-token"]');
    expect(meta?.getAttribute('content')).toBe('meta-token-123');
  });

  it('getCsrfToken prefers meta tag over cookie', () => {
    updateCsrfTokenMeta('meta-token');
    expect(getCsrfToken()).toBe('meta-token');
  });

  it('refreshCsrfToken fetches token from response header', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      headers: {
        get: (name: string) => (name === 'X-CSRF-Token' ? 'fresh-token' : null),
      },
    } as Response);

    const token = await refreshCsrfToken();

    expect(token).toBe('fresh-token');
    expect(mockFetch).toHaveBeenCalledWith(CSRF_REFRESH_PATH, {
      method: 'GET',
      credentials: 'same-origin',
    });
    expect(getCsrfToken()).toBe('fresh-token');
  });

  it('refreshCsrfToken dispatches refresh event', async () => {
    const listener = vi.fn();
    window.addEventListener(CSRF_TOKEN_REFRESHED_EVENT, listener);

    mockFetch.mockResolvedValueOnce({
      ok: true,
      headers: {
        get: () => 'event-token',
      },
    } as Response);

    await refreshCsrfToken();
    expect(listener).toHaveBeenCalled();
    window.removeEventListener(CSRF_TOKEN_REFRESHED_EVENT, listener);
  });

  it('refreshCsrfToken throws when header is missing', async () => {
    mockFetch.mockResolvedValueOnce({
      ok: true,
      headers: { get: () => null },
    } as Response);

    await expect(refreshCsrfToken()).rejects.toThrow('CSRF token missing');
  });
});
