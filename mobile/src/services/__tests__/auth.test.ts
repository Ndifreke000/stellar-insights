/**
 * Tests for the secure auth storage (issue #2628).
 *
 * Verifies that:
 *  1. Tokens are persisted via react-native-keychain (NOT AsyncStorage).
 *  2. AsyncStorage is never written to for secrets — regression guard.
 *  3. `hasValidToken` correctly handles missing/expired tokens.
 *  4. `clearAuthTokens` removes the keychain entry.
 *  5. `refreshAuthTokens` calls the backend and stores new tokens.
 */

import * as Keychain from 'react-native-keychain';
import AsyncStorage from '@react-native-async-storage/async-storage';

// ── Mocks ─────────────────────────────────────────────────────────────────
jest.mock('react-native-keychain');
jest.mock('@react-native-async-storage/async-storage', () =>
  require('@react-native-async-storage/async-storage/jest/async-storage-mock'),
);
jest.mock('@utils/logger', () => ({
  logger: { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() },
}));

// Mock storage (MMKV)
// NOTE: We intentionally keep the factory self-contained (no outer variable
// references). `delete` is a reserved keyword and babel-plugin-jest-hoist's
// AST traversal does not recognise it as an identifier reference, so any outer
// variable used as that property's value would still be `undefined` when the
// factory runs.  Instead we get the mock references via require() after the
// mock is registered — a pattern that is immune to hoisting order.
jest.mock('@services/storage', () => ({
  storage: { getString: jest.fn(() => null), delete: jest.fn() },
}));
const _storageMock = require('@services/storage').storage as {
  getString: jest.Mock;
  delete: jest.Mock;
};
const mockStorageGetString = _storageMock.getString;
const mockStorageDelete = _storageMock.delete;

// Mock auth store
const mockSetTokens = jest.fn();
const mockSetUser = jest.fn();
const mockSetLoading = jest.fn();
const mockLogout = jest.fn();
jest.mock('@store/authStore', () => ({
  useAuthStore: {
    getState: () => ({
      setTokens: mockSetTokens,
      setUser: mockSetUser,
      setLoading: mockSetLoading,
      logout: mockLogout,
      tokens: { refreshToken: 'rt_mock' },
    }),
  },
}));

// Mock API client
const mockPost = jest.fn();
jest.mock('@services/api', () => ({ apiClient: { post: (...a: unknown[]) => mockPost(...a) } }));

// ── Cast keychain to jest mocks ────────────────────────────────────────────
const mockSetGenericPassword = Keychain.setGenericPassword as jest.Mock;
const mockGetGenericPassword = Keychain.getGenericPassword as jest.Mock;
const mockResetGenericPassword = Keychain.resetGenericPassword as jest.Mock;

import {
  saveToken,
  getToken,
  getTokenExpiry,
  hasValidToken,
  removeToken,
  loadStoredAuth,
  storeAuthTokens,
  clearAuthTokens,
  refreshAuthTokens,
} from '@services/auth';
import { logger } from '@utils/logger';

const AUTH_SERVICE = 'com.payraider.auth.tokens';

beforeEach(() => {
  jest.clearAllMocks();
  AsyncStorage.clear();
  // Reset MMKV getString to return null (no user data) by default.
  // Must be re-applied after clearAllMocks() which wipes all mockReturnValues.
  mockStorageGetString.mockReturnValue(null);
});

// ---------------------------------------------------------------------------
// 1. Secrets go to Keychain, NOT AsyncStorage
// ---------------------------------------------------------------------------
describe('secure storage — keychain (not AsyncStorage)', () => {
  it('saveToken writes to Keychain, not AsyncStorage', async () => {
    mockSetGenericPassword.mockResolvedValue(true);

    await saveToken('my-secret-token', Date.now() + 60_000);

    expect(mockSetGenericPassword).toHaveBeenCalledWith(
      AUTH_SERVICE,
      expect.any(String), // JSON string — we do NOT inspect the token value
      { service: AUTH_SERVICE },
    );

    // REGRESSION GUARD: AsyncStorage must never be written to for secrets
    const allKeys = await AsyncStorage.getAllKeys();
    const secretKeys = allKeys.filter(k => k.includes('token') || k.includes('auth'));
    expect(secretKeys).toHaveLength(0);
  });

  it('getToken reads from Keychain', async () => {
    const stored = JSON.stringify({ tokens: { accessToken: 'tk_test' } });
    mockGetGenericPassword.mockResolvedValue({ password: stored, username: AUTH_SERVICE });

    const result = await getToken();
    expect(result).toBe('tk_test');
    expect(mockGetGenericPassword).toHaveBeenCalledWith({ service: AUTH_SERVICE });
  });
});

// ---------------------------------------------------------------------------
// 3. hasValidToken
// ---------------------------------------------------------------------------
describe('hasValidToken', () => {
  it('returns false when no token is stored', async () => {
    mockGetGenericPassword.mockResolvedValue(false);
    expect(await hasValidToken()).toBe(false);
  });

  it('returns false when the token is expired', async () => {
    const stored = JSON.stringify({ tokens: { accessToken: 'tk' }, expiresAt: Date.now() - 1000 });
    mockGetGenericPassword.mockResolvedValue({ password: stored });
    expect(await hasValidToken()).toBe(false);
  });

  it('returns true when the token is present and not expired', async () => {
    const stored = JSON.stringify({ tokens: { accessToken: 'tk' }, expiresAt: Date.now() + 60_000 });
    mockGetGenericPassword.mockResolvedValue({ password: stored });
    expect(await hasValidToken()).toBe(true);
  });

  it('returns true when the token has no expiry (never-expires)', async () => {
    const stored = JSON.stringify({ tokens: { accessToken: 'tk' } });
    mockGetGenericPassword.mockResolvedValue({ password: stored });
    expect(await hasValidToken()).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// 4. clearAuthTokens
// ---------------------------------------------------------------------------
describe('clearAuthTokens', () => {
  it('resets the keychain entry and calls store logout', async () => {
    mockResetGenericPassword.mockResolvedValue(true);

    await clearAuthTokens();

    expect(mockResetGenericPassword).toHaveBeenCalledWith({ service: AUTH_SERVICE });
    expect(mockStorageDelete).toHaveBeenCalled(); // cached user profile is wiped too
    expect(mockLogout).toHaveBeenCalled();
  });
});

// ---------------------------------------------------------------------------
// getTokenExpiry / removeToken
// ---------------------------------------------------------------------------
describe('getTokenExpiry and removeToken', () => {
  it('returns the stored expiry, or null when none is set', async () => {
    const expiresAt = Date.now() + 60_000;
    mockGetGenericPassword.mockResolvedValueOnce({
      password: JSON.stringify({ tokens: { accessToken: 'tk' }, expiresAt }),
    });
    expect(await getTokenExpiry()).toBe(expiresAt);

    mockGetGenericPassword.mockResolvedValueOnce({
      password: JSON.stringify({ tokens: { accessToken: 'tk' } }),
    });
    expect(await getTokenExpiry()).toBeNull();
  });

  it('removeToken resets the keychain entry', async () => {
    mockResetGenericPassword.mockResolvedValue(true);
    await removeToken();
    expect(mockResetGenericPassword).toHaveBeenCalledWith({ service: AUTH_SERVICE });
  });
});

// ---------------------------------------------------------------------------
// 2. storeAuthTokens
// ---------------------------------------------------------------------------
describe('storeAuthTokens', () => {
  it('writes the full AuthTokens object to keychain', async () => {
    const tokens = { accessToken: 'at_abc', refreshToken: 'rt_abc', expiresAt: 1_900_000_000_000 };
    mockSetGenericPassword.mockResolvedValue(true);

    await storeAuthTokens(tokens);

    // auth.ts wraps tokens as { tokens, expiresAt? } so the refresh token is
    // never lost when only the access token is updated via saveToken().
    expect(mockSetGenericPassword).toHaveBeenCalledWith(
      AUTH_SERVICE,
      JSON.stringify({ tokens, expiresAt: tokens.expiresAt }),
      { service: AUTH_SERVICE },
    );
  });

  it('hydrates the auth store via setTokens', async () => {
    const tokens = { accessToken: 'at_xyz', refreshToken: 'rt_xyz', expiresAt: 1_900_000_000_000 };
    mockSetGenericPassword.mockResolvedValue(true);

    await storeAuthTokens(tokens);

    expect(mockSetTokens).toHaveBeenCalledWith(tokens);
  });
});

// ---------------------------------------------------------------------------
// 6. loadStoredAuth
// ---------------------------------------------------------------------------
describe('loadStoredAuth', () => {
  it('sets tokens in store when keychain has tokens', async () => {
    const tokens = { accessToken: 'at_stored', refreshToken: 'rt_stored' };
    mockGetGenericPassword.mockResolvedValue({ password: JSON.stringify({ tokens }), username: AUTH_SERVICE });

    await loadStoredAuth();

    expect(mockSetTokens).toHaveBeenCalledWith(tokens);
  });

  it('also sets user in store when MMKV has user data', async () => {
    const tokens = { accessToken: 'at_stored', refreshToken: 'rt_stored' };
    const user = { id: 'u1', email: 'alice@example.com' };
    mockGetGenericPassword.mockResolvedValue({ password: JSON.stringify({ tokens }), username: AUTH_SERVICE });
    mockStorageGetString.mockReturnValue(JSON.stringify(user));

    await loadStoredAuth();

    expect(mockSetTokens).toHaveBeenCalledWith(tokens);
    expect(mockSetUser).toHaveBeenCalledWith(user);
  });

  it('does not call setTokens when keychain is empty', async () => {
    mockGetGenericPassword.mockResolvedValue(false);

    await loadStoredAuth();

    expect(mockSetTokens).not.toHaveBeenCalled();
    expect(mockSetUser).not.toHaveBeenCalled();
  });

  it('always calls setLoading(false) in the finally block', async () => {
    mockGetGenericPassword.mockResolvedValue(false);

    await loadStoredAuth();

    expect(mockSetLoading).toHaveBeenCalledWith(false);
  });

  it('calls setLoading(false) and logs an error when keychain throws', async () => {
    mockGetGenericPassword.mockRejectedValue(new Error('Keychain unavailable'));

    await loadStoredAuth();

    expect(mockSetLoading).toHaveBeenCalledWith(false);
    expect(mockSetTokens).not.toHaveBeenCalled();
    expect((logger.error as jest.Mock)).toHaveBeenCalledWith(
      '[Auth]',
      'Failed to load stored auth:',
      expect.any(Error),
    );
  });
});

// ---------------------------------------------------------------------------
// 5. refreshAuthTokens
// ---------------------------------------------------------------------------
describe('refreshAuthTokens', () => {
  it('fetches new tokens and persists them', async () => {
    const newTokens = { accessToken: 'new_at', refreshToken: 'new_rt' };
    mockPost.mockResolvedValue(newTokens);
    mockSetGenericPassword.mockResolvedValue(true);

    const result = await refreshAuthTokens();

    expect(result).toEqual(newTokens);
    expect(mockPost).toHaveBeenCalledWith('/auth/refresh', { refreshToken: 'rt_mock' });
    expect(mockSetGenericPassword).toHaveBeenCalledWith(
      AUTH_SERVICE,
      JSON.stringify({ tokens: newTokens }),
      { service: AUTH_SERVICE },
    );
  });

  it('returns null without touching stored tokens when refresh fails', async () => {
    // Contract: the caller (api.ts response interceptor) decides whether to
    // re-authenticate; a failed refresh must not throw or silently persist.
    mockPost.mockRejectedValue(new Error('401 Unauthorized'));

    const result = await refreshAuthTokens();

    expect(result).toBeNull();
    expect(mockSetGenericPassword).not.toHaveBeenCalled();
    expect((logger.error as jest.Mock)).toHaveBeenCalledWith(
      '[Auth]',
      'Failed to refresh tokens:',
      expect.any(Error),
    );
  });
});
