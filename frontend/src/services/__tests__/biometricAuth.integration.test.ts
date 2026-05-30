/**
 * Integration tests for biometric auth post-update recovery flow
 *
 * Simulates the complete user journey:
 *   1. User has registered biometric credential (v1)
 *   2. App updates, version changes to v2
 *   3. User tries to authenticate → gets STALE_CREDENTIAL error
 *   4. UI prompts to re-register
 *   5. User confirms re-registration
 *   6. New credential created with v2
 *   7. Next login succeeds
 */

import { describe, it, expect, beforeEach, vi, type MockedFunction } from 'vitest';
import {
  CREDENTIAL_VERSION,
  CREDENTIAL_EXPIRY_MS,
  isStaleCredential,
  loadCredential,
  clearCredential,
  registerBiometric,
  authenticateWithBiometric,
  type StoredCredential,
} from '../biometricAuth';

// Mock WebAuthn APIs
const mockCreate = vi.fn();
const mockGet = vi.fn();

Object.defineProperty(global, 'navigator', {
  value: { credentials: { create: mockCreate, get: mockGet } },
  writable: true,
});

Object.defineProperty(global, 'PublicKeyCredential', {
  value: {
    isUserVerifyingPlatformAuthenticatorAvailable: vi.fn().mockResolvedValue(true),
  },
  writable: true,
});

Object.defineProperty(global, 'crypto', {
  value: { getRandomValues: (arr: Uint8Array) => arr.fill(1) },
  writable: true,
});

Object.defineProperty(global, 'window', {
  value: { location: { hostname: 'example.com' }, PublicKeyCredential: {} },
  writable: true,
});

// Mock localStorage
const store: Record<string, string> = {};
const localStorageMock = {
  getItem: (k: string) => store[k] ?? null,
  setItem: (k: string, v: string) => { store[k] = v; },
  removeItem: (k: string) => { delete store[k]; },
};
Object.defineProperty(global, 'localStorage', { value: localStorageMock });

// Helpers
function makeMockCredential(rawIdBytes = new Uint8Array([1, 2, 3])) {
  return {
    rawId: rawIdBytes.buffer,
    type: 'public-key',
    response: {},
  } as unknown as PublicKeyCredential;
}

function seedOldVersionCredential(userId: string) {
  const old: StoredCredential = {
    credentialId: 'AQID',
    userId,
    version: CREDENTIAL_VERSION - 1, // old version
    registeredAt: Date.now() - 1_000_000,
    expiresAt: Date.now() + 1_000_000,
    appVersion: '1.0.0',
  };
  store[`biometric_credential:${userId}`] = JSON.stringify(old);
}

describe('Biometric Auth Post-Update Recovery Flow', () => {
  beforeEach(() => {
    Object.keys(store).forEach(k => delete store[k]);
    mockCreate.mockReset();
    mockGet.mockReset();
  });

  it('detects stale credential after version bump and allows re-registration', async () => {
    const userId = 'user@example.com';

    // Step 1: Simulate user having an old credential
    seedOldVersionCredential(userId);
    expect(isStaleCredential(userId)).toBe(true);

    // Step 2: User tries to authenticate with stale credential → fails
    const authResult = await authenticateWithBiometric(userId);
    expect(authResult.ok).toBe(false);
    if (!authResult.ok) expect(authResult.error).toBe('STALE_CREDENTIAL');

    // Step 3: UI detects stale credential and nudges user to re-register
    expect(isStaleCredential(userId)).toBe(true);

    // Step 4: User initiates re-registration
    mockCreate.mockResolvedValueOnce(makeMockCredential());
    const registerResult = await registerBiometric(userId, 'Test User');
    expect(registerResult.ok).toBe(true);

    // Step 5: New credential has current version
    const newCred = loadCredential(userId);
    expect(newCred?.version).toBe(CREDENTIAL_VERSION);
    expect(newCred?.appVersion).toBeDefined();
    expect(newCred?.expiresAt).toBeDefined();
    expect(isStaleCredential(userId)).toBe(false);

    // Step 6: Next authentication succeeds
    mockGet.mockResolvedValueOnce(makeMockCredential());
    const nextAuthResult = await authenticateWithBiometric(userId);
    expect(nextAuthResult.ok).toBe(true);
  });

  it('handles SecurityError as stale credential (rpId/origin mismatch)', async () => {
    const userId = 'user@example.com';

    // Seed current version credential
    const cred: StoredCredential = {
      credentialId: 'AQID',
      userId,
      version: CREDENTIAL_VERSION,
      registeredAt: Date.now(),
      expiresAt: Date.now() + CREDENTIAL_EXPIRY_MS,
      appVersion: '2.0.0',
    };
    store[`biometric_credential:${userId}`] = JSON.stringify(cred);

    // Simulate SecurityError (e.g., domain changed from example.com to example.co.uk)
    mockGet.mockRejectedValueOnce(new DOMException('rpId mismatch', 'SecurityError'));

    const result = await authenticateWithBiometric(userId);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe('STALE_CREDENTIAL');
    }

    // Credential should be cleared so user can re-register
    expect(loadCredential(userId)).toBeNull();

    // Re-registration path
    mockCreate.mockResolvedValueOnce(makeMockCredential());
    const registerResult = await registerBiometric(userId, 'Test User');
    expect(registerResult.ok).toBe(true);
  });

  it('enforces credential expiry after 90 days', async () => {
    const userId = 'user@example.com';

    // Seed credential that's 91 days old
    const expired: StoredCredential = {
      credentialId: 'AQID',
      userId,
      version: CREDENTIAL_VERSION,
      registeredAt: Date.now() - CREDENTIAL_EXPIRY_MS - 86_400_000, // 91 days
      expiresAt: Date.now() - 86_400_000, // expired 1 day ago
      appVersion: '2.0.0',
    };
    store[`biometric_credential:${userId}`] = JSON.stringify(expired);

    // Authentication should fail with STALE_CREDENTIAL
    const result = await authenticateWithBiometric(userId);
    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error).toBe('STALE_CREDENTIAL');
    }

    // Credential should be cleared
    expect(loadCredential(userId)).toBeNull();

    // User should be able to re-register
    mockCreate.mockResolvedValueOnce(makeMockCredential());
    const registerResult = await registerBiometric(userId, 'Test User');
    expect(registerResult.ok).toBe(true);

    const newCred = loadCredential(userId);
    expect(newCred?.expiresAt).toBeDefined();
    expect(newCred!.expiresAt! - newCred!.registeredAt).toBe(CREDENTIAL_EXPIRY_MS);
  });

  it('multi-user scenario: each user credential versioned independently', async () => {
    const user1 = 'user1@example.com';
    const user2 = 'user2@example.com';

    // User1 has old credential, User2 has current credential
    seedOldVersionCredential(user1);

    const user2Cred: StoredCredential = {
      credentialId: 'AQID',
      userId: user2,
      version: CREDENTIAL_VERSION,
      registeredAt: Date.now(),
      expiresAt: Date.now() + CREDENTIAL_EXPIRY_MS,
      appVersion: '2.0.0',
    };
    store[`biometric_credential:${user2}`] = JSON.stringify(user2Cred);

    // User1: stale credential
    expect(isStaleCredential(user1)).toBe(true);

    // User2: current credential
    expect(isStaleCredential(user2)).toBe(false);

    // User2 can authenticate successfully
    mockGet.mockResolvedValueOnce(makeMockCredential());
    const user2Result = await authenticateWithBiometric(user2);
    expect(user2Result.ok).toBe(true);

    // User1 must re-register
    const user1AuthResult = await authenticateWithBiometric(user1);
    expect(user1AuthResult.ok).toBe(false);
    if (!user1AuthResult.ok) {
      expect(user1AuthResult.error).toBe('STALE_CREDENTIAL');
    }
  });
});
