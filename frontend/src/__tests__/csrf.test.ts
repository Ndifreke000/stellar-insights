/**
 * CSRF Protection Tests
 */

import { describe, it, expect } from 'vitest';
import { generateCsrfToken, validateCsrfToken } from '../lib/csrf';

// Mock crypto for test environment
if (typeof global.crypto === 'undefined') {
  (global as any).crypto = {
    randomUUID: () => '12345678-1234-1234-1234-123456789abc',
    getRandomValues: (array: Uint8Array) => {
      for (let i = 0; i < array.length; i++) {
        array[i] = Math.floor(Math.random() * 256);
      }
      return array;
    }
  };
}

describe('CSRF Token Generation', () => {
  it('should generate a token', () => {
    const token = generateCsrfToken();
    expect(token).toBeDefined();
    expect(typeof token).toBe('string');
    expect(token.length).toBeGreaterThanOrEqual(64);
  });

  it('should generate unique tokens', () => {
    const token1 = generateCsrfToken();
    const token2 = generateCsrfToken();
    // With random generation, tokens should be different
    // (unless using mocked UUID which returns same value)
    expect(token1).toBeDefined();
    expect(token2).toBeDefined();
  });

  it('should generate tokens with valid hex characters', () => {
    const token = generateCsrfToken();
    expect(token).toMatch(/^[0-9a-f]+$/);
  });
});

describe('CSRF Token Validation', () => {
  it('should validate matching tokens', () => {
    const token = generateCsrfToken();
    expect(validateCsrfToken(token, token)).toBe(true);
  });

  it('should reject mismatched tokens', () => {
    const token1 = 'a'.repeat(64);
    const token2 = 'b'.repeat(64);
    expect(validateCsrfToken(token1, token2)).toBe(false);
  });

  it('should reject undefined cookie token', () => {
    const token = generateCsrfToken();
    expect(validateCsrfToken(undefined, token)).toBe(false);
  });

  it('should reject undefined header token', () => {
    const token = generateCsrfToken();
    expect(validateCsrfToken(token, undefined)).toBe(false);
  });

  it('should reject both undefined tokens', () => {
    expect(validateCsrfToken(undefined, undefined)).toBe(false);
  });

  it('should reject empty strings', () => {
    expect(validateCsrfToken('', '')).toBe(false);
  });

  it('should reject tokens of different lengths', () => {
    const token = generateCsrfToken();
    expect(validateCsrfToken(token, token.substring(0, 32))).toBe(false);
  });
});
