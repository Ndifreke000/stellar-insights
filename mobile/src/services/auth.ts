/**
 * Centralised auth-token store for Stellar Insights mobile.
 *
 * Previously split across `auth.ts` (expo-secure-store, key "…tokens") and
 * `tokenStorage.ts` (expo-secure-store, key "…token"), causing the login screen
 * and the API interceptor/boot restore to read from different keys.  Both files
 * used expo-secure-store, which requires expo-modules-core and cannot be
 * auto-linked in a bare React Native project.
 *
 * This file is now the **single** token storage module:
 *  - Uses `react-native-keychain` (already a declared dependency, works in
 *    bare RN with no Expo toolchain required).
 *  - Single keychain service name: `com.stellarinsights.auth.tokens`.
 *  - Re-exports the `tokenStorage.ts` API surface
 *    (`saveToken`, `getToken`, `getTokenExpiry`, `removeToken`, `clearAll`,
 *    `hasValidToken`) so existing call sites in `App.tsx` and
 *    `useLoginScreen.ts` compile without changes.
 *  - Exposes the full `AuthTokens`-aware helpers previously in this file
 *    (`loadStoredAuth`, `storeAuthTokens`, `clearAuthTokens`,
 *    `refreshAuthTokens`).
 */

import * as Keychain from 'react-native-keychain';
import { STORAGE_KEYS } from '@config/constants';
import { useAuthStore } from '@store/authStore';
import { AuthTokens, User } from '@app-types/index';
import { storage } from './storage';

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

/** Keychain service name — every read and write in this file uses this value. */
const SERVICE = 'com.stellarinsights.auth.tokens';

/**
 * Key within the stored JSON payload that carries the optional token expiry
 * (Unix epoch, ms).  This parallels the old tokenStorage.ts EXPIRY_KEY.
 */
const EXPIRY_FIELD = 'expiresAt';

// ---------------------------------------------------------------------------
// Low-level keychain helpers (not exported)
// ---------------------------------------------------------------------------

/** Persist an arbitrary JSON-serialisable value in the keychain. */
async function _keychainSet(value: unknown): Promise<void> {
  await Keychain.setGenericPassword(
    SERVICE,
    JSON.stringify(value),
    { service: SERVICE },
  );
}

/** Read and JSON-parse the keychain value, or return `null`. */
async function _keychainGet<T>(): Promise<T | null> {
  const result = await Keychain.getGenericPassword({ service: SERVICE });
  if (!result) return null;
  try {
    return JSON.parse(result.password) as T;
  } catch {
    return null;
  }
}

/** Delete the keychain entry entirely. */
async function _keychainDelete(): Promise<void> {
  await Keychain.resetGenericPassword({ service: SERVICE });
}

// ---------------------------------------------------------------------------
// Internal shape stored in the keychain
// ---------------------------------------------------------------------------

interface StoredCredentials {
  /** Full AuthTokens object (accessToken, refreshToken, …). */
  tokens: AuthTokens;
  /**
   * Unix epoch in milliseconds at which the access token expires.
   * Stored here so `hasValidToken` / `getTokenExpiry` can avoid a full
   * AuthTokens parse in the simple-check path.
   */
  [EXPIRY_FIELD]?: number;
}

// ---------------------------------------------------------------------------
// AuthTokens-aware API  (used by initialization.ts, api.ts, sep10.ts)
// ---------------------------------------------------------------------------

/**
 * On app boot: read stored tokens from the keychain and rehydrate the auth
 * store.  Called by `initialization.ts`.
 */
export async function loadStoredAuth(): Promise<void> {
  try {
    const stored = await _keychainGet<StoredCredentials>();
    if (stored?.tokens) {
      useAuthStore.getState().setTokens(stored.tokens);

      const userData = storage.getString(STORAGE_KEYS.USER_DATA);
      if (userData) {
        const user: User = JSON.parse(userData);
        useAuthStore.getState().setUser(user);
      }
    }
  } catch (error) {
    console.error('Failed to load stored auth:', error);
  } finally {
    useAuthStore.getState().setLoading(false);
  }
}

/**
 * Persist a full `AuthTokens` object and sync the auth store.
 * Called after a successful login or token refresh.
 */
export async function storeAuthTokens(tokens: AuthTokens): Promise<void> {
  const payload: StoredCredentials = {
    tokens,
    ...(tokens.expiresAt !== undefined && { [EXPIRY_FIELD]: tokens.expiresAt }),
  };
  await _keychainSet(payload);
  useAuthStore.getState().setTokens(tokens);
}

/**
 * Delete all stored tokens and clear the auth store.
 * Called on logout.
 */
export async function clearAuthTokens(): Promise<void> {
  await _keychainDelete();
  storage.delete(STORAGE_KEYS.USER_DATA);
  useAuthStore.getState().logout();
}

/**
 * Use the stored refresh token to obtain a new token pair.
 * On success the new tokens are persisted; on failure the caller must handle
 * re-authentication.
 */
export async function refreshAuthTokens(): Promise<AuthTokens | null> {
  const { tokens } = useAuthStore.getState();
  if (!tokens?.refreshToken) return null;

  try {
    // Import lazily to avoid circular dependency (api.ts → auth.ts → api.ts).
    const { apiClient } = await import('./api');
    const newTokens = await apiClient.post<AuthTokens>('/auth/refresh', {
      refreshToken: tokens.refreshToken,
    });

    await storeAuthTokens(newTokens);
    return newTokens;
  } catch (error) {
    console.error('Failed to refresh tokens:', error);
    return null;
  }
}

// ---------------------------------------------------------------------------
// tokenStorage.ts-compatible API
// (App.tsx uses `hasValidToken`; useLoginScreen.ts uses `saveToken`/`getToken`)
// ---------------------------------------------------------------------------

/**
 * Persist an access token string together with an optional expiry timestamp.
 *
 * This is the call-site-compatible replacement for `tokenStorage.saveToken`.
 * Internally it merges the new access token into any existing StoredCredentials
 * so the full AuthTokens object is not lost.
 *
 * @param token      The access token string.
 * @param expiresAt  Optional Unix epoch in milliseconds.
 */
export async function saveToken(
  token: string,
  expiresAt?: number,
): Promise<void> {
  // Read existing stored credentials so we don't discard the refresh token.
  const existing = await _keychainGet<StoredCredentials>();
  const tokens: AuthTokens = {
    ...(existing?.tokens ?? {}),
    accessToken: token,
    ...(expiresAt !== undefined && { expiresAt }),
  } as AuthTokens;

  await storeAuthTokens(tokens);
}

/**
 * Read the stored access token string.
 *
 * This is the call-site-compatible replacement for `tokenStorage.getToken`.
 *
 * @returns The access token, or `null` when none is stored.
 */
export async function getToken(): Promise<string | null> {
  const stored = await _keychainGet<StoredCredentials>();
  return stored?.tokens?.accessToken ?? null;
}

/**
 * Read the stored expiry timestamp.
 *
 * This is the call-site-compatible replacement for `tokenStorage.getTokenExpiry`.
 *
 * @returns Unix epoch in milliseconds, or `null` when unset.
 */
export async function getTokenExpiry(): Promise<number | null> {
  const stored = await _keychainGet<StoredCredentials>();
  const raw = stored?.[EXPIRY_FIELD] ?? stored?.tokens?.expiresAt;
  if (raw === undefined || raw === null) return null;
  const parsed = Number(raw);
  return Number.isFinite(parsed) ? parsed : null;
}

/**
 * Remove the stored token (and metadata).
 *
 * This is the call-site-compatible replacement for `tokenStorage.removeToken`.
 */
export async function removeToken(): Promise<void> {
  await _keychainDelete();
}

/**
 * Alias for `removeToken` — clears the entire keychain entry.
 *
 * This is the call-site-compatible replacement for `tokenStorage.clearAll`.
 */
export const clearAll = removeToken;

/**
 * Return `true` when a non-expired access token is present.
 *
 * This is the call-site-compatible replacement for `tokenStorage.hasValidToken`,
 * and is also the function `App.tsx` calls to decide the initial route.
 *
 * @returns `true` if a valid (present and unexpired) access token is stored.
 */
export async function hasValidToken(): Promise<boolean> {
  const token = await getToken();
  if (!token) return false;

  const expiry = await getTokenExpiry();
  if (expiry !== null && expiry <= Date.now()) return false;

  return true;
}
