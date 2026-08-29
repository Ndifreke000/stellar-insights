/**
 * Tests for the token-storage API re-exported by auth.ts.
 *
 * Previously this file tested `tokenStorage.ts` (deleted) which used
 * expo-secure-store.  auth.ts is the single token store; it exposes the same
 * public API and uses react-native-keychain instead.
 */

import {
  clearAll,
  getToken,
  getTokenExpiry,
  hasValidToken,
  removeToken,
  saveToken,
} from '@services/auth';

// ---------------------------------------------------------------------------
// Mock react-native-keychain
// ---------------------------------------------------------------------------

// The keychain is keyed by service name.  We simulate a single-entry store
// because auth.ts uses one service ('com.stellarinsights.auth.tokens').
const keychainStore: Record<string, string> = {};

jest.mock('react-native-keychain', () => ({
  setGenericPassword: jest.fn(async (_username: string, password: string, options?: { service?: string }) => {
    const key = options?.service ?? 'default';
    keychainStore[key] = password;
    return true;
  }),
  getGenericPassword: jest.fn(async (options?: { service?: string }) => {
    const key = options?.service ?? 'default';
    const password = keychainStore[key];
    if (!password) return false;
    return { username: key, password };
  }),
  resetGenericPassword: jest.fn(async (options?: { service?: string }) => {
    const key = options?.service ?? 'default';
    delete keychainStore[key];
    return true;
  }),
}));

// Also mock the storage module (used by loadStoredAuth → storage.getString)
// to avoid the "not initialised" proxy guard in tests that don't call initializeStorage.
jest.mock('@services/storage', () => ({
  storage: {
    getString: jest.fn(() => undefined),
    set: jest.fn(),
    delete: jest.fn(),
  },
  storageUtils: {
    getItem: jest.fn(() => undefined),
    setItem: jest.fn(),
    removeItem: jest.fn(),
    clear: jest.fn(),
  },
  initializeStorage: jest.fn(async () => {}),
}));

// Stub the auth store so setTokens / setLoading don't throw
jest.mock('@store/authStore', () => ({
  useAuthStore: {
    getState: jest.fn(() => ({
      setTokens: jest.fn(),
      setUser: jest.fn(),
      setLoading: jest.fn(),
      logout: jest.fn(),
      tokens: null,
    })),
    setState: jest.fn(),
  },
}));

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const SERVICE = 'com.stellarinsights.auth.tokens';

function clearKeychainStore() {
  Object.keys(keychainStore).forEach(k => delete keychainStore[k]);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('auth token storage API', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    clearKeychainStore();
  });

  it('saves the token and expiry to the keychain', async () => {
    const expiresAt = Date.now() + 60_000;
    await saveToken('my-token', expiresAt);

    const stored = JSON.parse(keychainStore[SERVICE]);
    expect(stored.tokens.accessToken).toBe('my-token');
    expect(stored.expiresAt).toBe(expiresAt);
  });

  it('saves a token without expiry (no expiresAt in payload)', async () => {
    await saveToken('my-token');

    const stored = JSON.parse(keychainStore[SERVICE]);
    expect(stored.tokens.accessToken).toBe('my-token');
    expect(stored.expiresAt).toBeUndefined();
  });

  it('returns the stored token value', async () => {
    await saveToken('my-token');
    await expect(getToken()).resolves.toBe('my-token');
  });

  it('returns null when no token is stored', async () => {
    await expect(getToken()).resolves.toBeNull();
  });

  it('reads and parses the expiry, returning null for missing values', async () => {
    // No token stored
    await expect(getTokenExpiry()).resolves.toBeNull();

    // Token with expiry
    const expiresAt = 987654321;
    await saveToken('tok', expiresAt);
    await expect(getTokenExpiry()).resolves.toBe(expiresAt);
  });

  it('removes the token', async () => {
    await saveToken('my-token');
    await removeToken();
    await expect(getToken()).resolves.toBeNull();
  });

  it('clearAll removes the token (alias for removeToken)', async () => {
    await saveToken('my-token');
    await clearAll();
    await expect(getToken()).resolves.toBeNull();
  });

  describe('hasValidToken', () => {
    it('is false when no token is stored', async () => {
      await expect(hasValidToken()).resolves.toBe(false);
    });

    it('is true when a token without expiry is stored', async () => {
      await saveToken('tok');
      await expect(hasValidToken()).resolves.toBe(true);
    });

    it('is true when a token with a future expiry is stored', async () => {
      await saveToken('tok', Date.now() + 60_000);
      await expect(hasValidToken()).resolves.toBe(true);
    });

    it('is false when the stored token has expired', async () => {
      await saveToken('tok', Date.now() - 60_000);
      await expect(hasValidToken()).resolves.toBe(false);
    });
  });
});
