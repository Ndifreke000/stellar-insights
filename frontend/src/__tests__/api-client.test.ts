/**
 * API Client Tests
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { apiGet, apiPost, apiPut, apiPatch, apiDelete } from '../lib/api-client';

// Mock fetch
global.fetch = vi.fn();

const mockFetch = vi.mocked(global.fetch);

// Mock document.cookie
Object.defineProperty(document, 'cookie', {
  writable: true,
  value: 'csrf-token=test-token-123',
});

describe('API Client', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('apiGet', () => {
    it('should make GET request without CSRF token', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ data: 'test' }),
      });

      await apiGet('/api/test');

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'GET',
          headers: expect.not.objectContaining({
            'X-CSRF-Token': expect.any(String),
          }),
        })
      );
    });
  });

  describe('apiPost', () => {
    it('should include CSRF token in POST request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await apiPost('/api/test', { data: 'test' });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'POST',
          headers: expect.objectContaining({
            'X-CSRF-Token': 'test-token-123',
          }),
        })
      );
    });

    it('should throw error when CSRF token is missing', async () => {
      Object.defineProperty(document, 'cookie', {
        writable: true,
        value: '',
      });

      await expect(apiPost('/api/test', { data: 'test' })).rejects.toThrow(
        'CSRF token not found'
      );

      // Restore cookie
      Object.defineProperty(document, 'cookie', {
        writable: true,
        value: 'csrf-token=test-token-123',
      });
    });
  });

  describe('apiPut', () => {
    it('should include CSRF token in PUT request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await apiPut('/api/test', { data: 'test' });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'PUT',
          headers: expect.objectContaining({
            'X-CSRF-Token': 'test-token-123',
          }),
        })
      );
    });
  });

  describe('apiPatch', () => {
    it('should include CSRF token in PATCH request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await apiPatch('/api/test', { data: 'test' });

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'PATCH',
          headers: expect.objectContaining({
            'X-CSRF-Token': 'test-token-123',
          }),
        })
      );
    });
  });

  describe('apiDelete', () => {
    it('should include CSRF token in DELETE request', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      await apiDelete('/api/test');

      expect(global.fetch).toHaveBeenCalledWith(
        '/api/test',
        expect.objectContaining({
          method: 'DELETE',
          headers: expect.objectContaining({
            'X-CSRF-Token': 'test-token-123',
          }),
        })
      );
    });
  });

  describe('Error Handling', () => {
    it('should handle 403 CSRF errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 403,
        json: async () => ({ error: 'Invalid CSRF token' }),
      });

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => 'refreshed-token' },
      } as Response);

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      const result = await apiPost('/api/test', { data: 'test' });

      expect(result).toEqual({ success: true });
      expect(mockFetch).toHaveBeenCalledTimes(3);
    });

    it('should refresh CSRF token and retry once on 401', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
      });

      mockFetch.mockResolvedValueOnce({
        ok: true,
        headers: { get: () => 'new-csrf-token' },
      } as Response);

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ success: true }),
      });

      const result = await apiPost('/api/test', { data: 'test' });

      expect(result).toEqual({ success: true });
      expect(mockFetch).toHaveBeenCalledTimes(3);
      expect(mockFetch.mock.calls[2][1]).toMatchObject({
        headers: expect.objectContaining({
          'X-CSRF-Token': 'new-csrf-token',
        }),
      });
    });

    it('should handle general API errors', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
      });

      await expect(apiGet('/api/test')).rejects.toThrow('API error: 500');
    });
  });
});
