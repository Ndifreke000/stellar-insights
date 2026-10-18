/**
 * Centralised auth-token store for PayRaider mobile.
 *
 * ## Security fix (issue #2628)
 *
 * The previous implementation used `expo-secure-store`, which:
 *  1. Requires the Expo modules native build chain — incompatible with a bare
 *     React Native project without additional setup.
 *  2. Stored tokens under a simple key string with no service-scoping,
 *     making collisions between apps on a shared device keychain possible.
 *
 * This file now uses `react-native-keychain` (already a declared dependency,
 * works in bare RN with no Expo toolchain required).  The keychain service is
 * namespaced to `com.payraider.auth.tokens` so it is isolated from every other
 * keychain entry on the device.
 *
 * ### Threat model closed by this change
 * **Attacker:** an app on the same physical device (or an automated script
 * running as the same user on a rooted/jailbroken device) that can enumerate
 * AsyncStorage or read unprotected file-system paths.
 * **What they could do before:** on a compromised device AsyncStorage is
 * readable without root; expo-secure-store on Android before API 23 fell back
 * to SharedPreferences (cleartext).
 * **What they can do after:** react-native-keychain uses Android Keystore /
 * iOS Keychain Services — hardware-backed on modern devices.  The plaintext
 * secret never leaves the secure enclave.
 *
 * ## API surface
 * Re-exports the `tokenStorage.ts` surface (`saveToken`, `getToken`,
 * `getTokenExpiry`, `removeToken`, `clearAll`, `hasValidToken`) so existing
 * call sites compile without changes.
 *
 * Also exposes the full `AuthTokens`-aware helpers:
 * `loadStoredAuth`, `storeAuthTokens`, `clearAuthTokens`, `refreshAuthTokens`.
 *
 * IMPORTANT: Never log the token value or Stellar secret seeds (S...) here or
 * in any caller.  Only log opaque, non-sensitive status strings.
 */

import * as Keychain from 'react-native-keychain';
import { STORAGE_KEYS } from '@config/constants';
import { useAuthStore } from '@store/authStore';
import { AuthTokens, User } from '@types/index';
import { apiClient } from './api';
import { storage } from './storage';
import { logger } from '@utils/logger';

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

/** Keychain service name — every read and write in this file uses this value. */
const AUTH_SERVICE = 'com.payraider.auth.tokens';

/**
 * Key within the stored JSON payload that carries the optional token expiry
 * (Unix epoch, ms).
 */
const EXPIRY_FIELD = 'expiresAt';

// ---------------------------------------------------------------------------
// Low-level keychain helpers (not exported)
// ---------------------------------------------------------------------------

/** Persist an arbitrary JSON-serialisable value in the keychain. */
async function _keychainSet(value: unknown): Promise<void> {
  await Keychain.setGenericPassword(
    AUTH_SERVICE,
    JSON.stringify(value),
    { service: AUTH_SERVICE },
  );
}

/** Read and JSON-parse the keychain value, or return `null`. */
async function _keychainGet<T>(): Promise<T | null> {
  const result = await Keychain.getGenericPassword({ service: AUTH_SERVICE });
  if (!result) return null;
  try {
    return JSON.parse(result.password) as T;
  } catch {
    return null;
  }
}

/** Delete the keychain entry entirely. */
async function _keychainDelete(): Promise<void> {
  await Keychain.resetGenericPassword({ service: AUTH_SERVICE });
}

// ---------------------------------------------------------------------------
// tokenStorage.ts-compatible API (saveToken / getToken / …)
// ---------------------------------------------------------------------------

/**
 * Shape stored in the keychain for the simple token-only surface.
 * Superset of AuthTokens so both APIs share the same keychain slot.
 */
interface StoredTokenPayload {
  token: string;
  [EXPIRY_FIELD]?: number;
}

/**
 * Persist an auth token in encrypted Keychain/Keystore storage.
 * Re-implements `tokenStorage.saveToken` without expo-secure-store.
 *
 * @param token    - The auth token to store.  Never log this value.
 * @param expiresAt - Optional expiry as a Unix epoch in milliseconds.
 */
export async function saveToken(token: string, expiresAt?: number): Promise<void> {
  const payload: StoredTokenPayload = { token };
  if (typeof expiresAt === 'number') {
    payload[EXPIRY_FIELD] = expiresAt;
  }
  await _keychainSet(payload);
}

/**
 * Read the stored auth token.
 * Re-implements `tokenStorage.getToken` without expo-secure-store.
 *
 * @returns The token, or `null` when none is stored.
 */
export async function getToken(): Promise<string | null> {
  const payload = await _keychainGet<StoredTokenPayload>();
  return payload?.token ?? null;
}

/**
 * Read the stored token expiry timestamp.
 * Re-implements `tokenStorage.getTokenExpiry`.
 *
 * @returns Expiry as a Unix epoch in milliseconds, or `null` when unset.
 */
export async function getTokenExpiry(): Promise<number | null> {
  const payload = await _keychainGet<StoredTokenPayload>();
  if (!payload) return null;
  const expiry = payload[EXPIRY_FIELD];
  return typeof expiry === 'number' && Number.isFinite(expiry) ? expiry : null;
}

/**
 * Remove the stored auth token and its expiry metadata.
 * Re-implements `tokenStorage.removeToken`.
 */
export async function removeToken(): Promise<void> {
  await _keychainDelete();
}

/**
 * Clear all auth-related storage (token + metadata).
 * Re-implements `tokenStorage.clearAll`.
 */
export async function clearAll(): Promise<void> {
  await _keychainDelete();
}

/**
 * Determine whether a usable (present and unexpired) token exists.
 * Re-implements `tokenStorage.hasValidToken`.
 *
 * @returns `true` when a non-expired token is stored.
 */
export async function hasValidToken(): Promise<boolean> {
  const token = await getToken();
  if (!token) return false;

  const expiry = await getTokenExpiry();
  if (expiry !== null && expiry <= Date.now()) return false;

  return true;
}

// ---------------------------------------------------------------------------
// AuthTokens-aware helpers
// ---------------------------------------------------------------------------

/**
 * Load previously persisted auth tokens from the keychain and hydrate the
 * Zustand auth store.  Called once at app boot by `initializeApp`.
 */
export async function loadStoredAuth(): Promise<void> {
  try {
    const tokens = await _keychainGet<AuthTokens>();
    if (tokens) {
      useAuthStore.getState().setTokens(tokens);

      const userData = storage.getString(STORAGE_KEYS.USER_DATA);
      if (userData) {
        const user: User = JSON.parse(userData);
        useAuthStore.getState().setUser(user);
      }
    }
  } catch (error) {
    // Log as warn — not critical; the user will simply be asked to log in.
    logger.warn('[Auth]', 'Failed to load stored auth — user will re-authenticate.');
  } finally {
    useAuthStore.getState().setLoading(false);
  }
}

/**
 * Persist a complete `AuthTokens` object in the keychain and update the store.
 * The token value itself is NEVER logged.
 */
export async function storeAuthTokens(tokens: AuthTokens): Promise<void> {
  await _keychainSet(tokens);
  useAuthStore.getState().setTokens(tokens);
}

/**
 * Remove auth tokens from the keychain, clear non-sensitive user data from
 * MMKV, and reset the auth store.
 */
export async function clearAuthTokens(): Promise<void> {
  await _keychainDelete();
  storage.delete(STORAGE_KEYS.USER_DATA);
  useAuthStore.getState().logout();
}

/**
 * Attempt a silent token refresh using the stored refresh token.
 * Returns `null` (and calls logout) when the refresh fails.
 * The token value itself is NEVER logged.
 */
export async function refreshAuthTokens(): Promise<AuthTokens | null> {
  const { tokens } = useAuthStore.getState();
  if (!tokens?.refreshToken) return null;

  try {
    const newTokens = await apiClient.post<AuthTokens>('/auth/refresh', {
      refreshToken: tokens.refreshToken,
    });

    await storeAuthTokens(newTokens);
    return newTokens;
  } catch (error) {
    logger.error('[Auth]', 'Token refresh failed — logging out.');
    await clearAuthTokens();
    return null;
  }
}
