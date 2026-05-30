/**
 * Regression tests for biometric auth post-update failures.
 *
 * Covers:
 *   - Stale credential detection after CREDENTIAL_VERSION bump
 *   - Credential expiry after 90 days (forced refresh)
 *   - SecurityError (rpId/origin mismatch) mapped to STALE_CREDENTIAL
 *   - Successful registration and authentication happy paths
 *   - All BiometricError classifications
 *   - Storage helpers (save / load / clear)
 *   - Post-update recovery flow (detection → UI nudge → re-registration)
 */

import { describe, it, expect, beforeEach, vi, type MockedFunction } from 'vitest';
import {
  CREDENTIAL_VERSION,
  CREDENTIAL_EXPIRY_MS,
  isBiometricSupported,
  isStaleCredential,
  loadCredential,
  clearCredential,
  registerBiometric,
  authenticateWithBiometric,
  type StoredCredential,
} from '../biometricAuth';

// ─── Browser API mocks ───────────────────────────────────────────────────────

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
  value: { location: { hostname: 'localhost' }, PublicKeyCredential: {} },
  writable: true,
});

// ─── localStorage mock ───────────────────────────────────────────────────────

const store: Record<string, string> = {};
const localStorageMock = {
  getItem: (k: string) => store[k] ?? null,
  setItem: (k: string, v: string) => { store[k] = v; },
  removeItem: (k: string) => { delete store[k]; },
};
Object.defineProperty(global, 'localStorage', { value: localStorageMock });

// ─── Helpers ─────────────────────────────────────────────────────────────────

function makeMockCredential(rawIdBytes = new Uint8Array([1, 2, 3])) {
  return {
    rawId: rawIdBytes.buffer,
    type: 'public-key',
    response: {},
  } as unknown as PublicKeyCredential;
}

function seedStaleCredential(userId: string) {
  const stale: StoredCredential = {
    credentialId: 'AQID', // base64url of [1,2,3]
    userId,
    version: CREDENTIAL_VERSION - 1,
    registeredAt: Date.now() - 10_000,
    expiresAt: Date.now() + 1_000_000,
    appVersion: '1.0.0',
  };
  localStorageMock.setItem(`biometric_credential:${userId}`, JSON.stringify(stale));
}

function seedCurrentCredential(userId: string) {
  const current: StoredCredential = {
    credentialId: 'AQID',
    userId,
    version: CREDENTIAL_VERSION,
    registeredAt: Date.now(),
    expiresAt: Date.now() + CREDENTIAL_EXPIRY_MS,
    appVersion: '2.0.0',
  };
  localStorageMock.setItem(`biometric_credential:${userId}`, JSON.stringify(current));
}

function seedExpiredCredential(userId: string) {
  const expired: StoredCredential = {
    credentialId: 'AQID',
    userId,
    version: CREDENTIAL_VERSION,
    registeredAt: Date.now() - CREDENTIAL_EXPIRY_MS - 1000,
    expiresAt: Date.now() - 1000, // expired 1 second ago
    appVersion: '2.0.0',
  };
  localStorageMock.setItem(`biometric_credential:${userId}`, JSON.stringify(expired));
}

// ─── Tests ───────────────────────────────────────────────────────────────────

describe('isBiometricSupported', () => {
  it('returns true when WebAuthn APIs are present', () => {
    expect(isBiometricSupported()).toBe(true);
  });
});

describe('isStaleCredential', () => {
  beforeEach(() => { Object.keys(store).forEach(k => delete store[k]); });

  it('returns false when no credential is stored', () => {
    expect(isStaleCredential('user1')).toBe(false);
  });

  it('returns true when stored version is below CREDENTIAL_VERSION', () => {
    seedStaleCredential('user1');
    expect(isStaleCredential('user1')).toBe(true);
  });

  it('returns false when stored version matches CREDENTIAL_VERSION', () => {
    seedCurrentCredential('user1');
    expect(isStaleCredential('user1')).toBe(false);
  });

  it('returns true when credential has expired (90+ days old)', () => {
    seedExpiredCredential('user1');
    expect(isStaleCredential('user1')).toBe(true);
  });
});

describe('loadCredential / clearCredential', () => {
  beforeEach(() => { Object.keys(store).forEach(k => delete store[k]); });

  it('returns null when nothing is stored', () => {
    expect(loadCredential('user1')).toBeNull();
  });

  it('returns the stored credential', () => {
    seedCurrentCredential('user1');
    const cred = loadCredential('user1');
    expect(cred).not.toBeNull();
    expect(cred?.userId).toBe('user1');
    expect(cred?.version).toBe(CREDENTIAL_VERSION);
  });

  it('clears the credential', () => {
    seedCurrentCredential('user1');
    clearCredential('user1');
    expect(loadCredential('user1')).toBeNull();
  });
});

describe('registerBiometric', () => {
  beforeEach(() => {
    Object.keys(store).forEach(k => delete store[k]);
    mockCreate.mockReset();
  });

  it('returns NOT_SUPPORTED when WebAuthn is unavailable', async () => {
    vi.spyOn(global, 'window', 'get').mockReturnValueOnce(undefined as never);
    const result = await registerBiometric('user1', 'User One');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('NOT_SUPPORTED');
  });

  it('registers successfully and persists credential with expiry and app version', async () => {
    mockCreate.mockResolvedValueOnce(makeMockCredential());
    const result = await registerBiometric('user1', 'User One');
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.userId).toBe('user1');
      expect(result.data.version).toBe(CREDENTIAL_VERSION);
      expect(result.data.expiresAt).toBeDefined();
      expect(result.data.appVersion).toBeDefined();
      // Expiry should be ~90 days in future
      const expiryDays = Math.floor((result.data.expiresAt! - result.data.registeredAt) / (1000 * 60 * 60 * 24));
      expect(expiryDays).toBe(90);
    }
    expect(loadCredential('user1')).not.toBeNull();
  });

  it('clears stale credential before re-registering (post-update fix)', async () => {
    seedStaleCredential('user1');
    expect(isStaleCredential('user1')).toBe(true);

    mockCreate.mockResolvedValueOnce(makeMockCredential());
    const result = await registerBiometric('user1', 'User One');

    expect(result.ok).toBe(true);
    // New credential must have current version
    const stored = loadCredential('user1');
    expect(stored?.version).toBe(CREDENTIAL_VERSION);
    expect(stored?.expiresAt).toBeDefined();
  });

  it('maps NotAllowedError to USER_CANCELLED', async () => {
    const err = new DOMException('cancelled', 'NotAllowedError');
    mockCreate.mockRejectedValueOnce(err);
    const result = await registerBiometric('user1', 'User One');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('USER_CANCELLED');
  });

  it('maps InvalidStateError to INVALID_STATE', async () => {
    const err = new DOMException('duplicate', 'InvalidStateError');
    mockCreate.mockRejectedValueOnce(err);
    const result = await registerBiometric('user1', 'User One');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('INVALID_STATE');
  });

  it('maps SecurityError to SECURITY_ERROR', async () => {
    const err = new DOMException('rpId mismatch', 'SecurityError');
    mockCreate.mockRejectedValueOnce(err);
    const result = await registerBiometric('user1', 'User One');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('SECURITY_ERROR');
  });
});

describe('authenticateWithBiometric', () => {
  beforeEach(() => {
    Object.keys(store).forEach(k => delete store[k]);
    mockGet.mockReset();
  });

  it('returns NOT_ENROLLED when no credential is stored', async () => {
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('NOT_ENROLLED');
  });

  it('returns STALE_CREDENTIAL when stored version is outdated (post-update regression)', async () => {
    seedStaleCredential('user1');
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('STALE_CREDENTIAL');
    // Must NOT call the authenticator — no point prompting the user
    expect(mockGet).not.toHaveBeenCalled();
  });

  it('returns STALE_CREDENTIAL when credential has expired', async () => {
    seedExpiredCredential('user1');
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('STALE_CREDENTIAL');
    // Expired credential must be cleared
    expect(loadCredential('user1')).toBeNull();
    expect(mockGet).not.toHaveBeenCalled();
  });

  it('authenticates successfully with a current credential', async () => {
    seedCurrentCredential('user1');
    mockGet.mockResolvedValueOnce(makeMockCredential());
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.data.credentialId).toBeTruthy();
      expect(result.data.timestamp).toBeGreaterThan(0);
    }
  });

  it('maps USER_CANCELLED correctly', async () => {
    seedCurrentCredential('user1');
    mockGet.mockRejectedValueOnce(new DOMException('cancelled', 'NotAllowedError'));
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('USER_CANCELLED');
  });

  it('maps SecurityError to STALE_CREDENTIAL and clears stored credential (post-update fix)', async () => {
    seedCurrentCredential('user1');
    mockGet.mockRejectedValueOnce(new DOMException('rpId mismatch', 'SecurityError'));
    const result = await authenticateWithBiometric('user1');
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.error).toBe('STALE_CREDENTIAL');
    // Credential must be cleared so UI can offer re-registration
    expect(loadCredential('user1')).toBeNull();
  });
});
