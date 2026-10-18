/**
 * @deprecated Use `@services/auth` instead.
 *
 * This file is a migration shim.  It re-exports the same API that the old
 * `tokenStorage.ts` (expo-secure-store based) provided so that existing import
 * sites continue to compile while we gradually update call sites to import
 * directly from `auth.ts`.
 *
 * Issue #2628 — Secrets in AsyncStorage / expo-secure-store are readable on
 * compromised devices.  The underlying implementation now uses
 * `react-native-keychain` (hardware-backed Keychain / Keystore).
 */
export {
  saveToken,
  getToken,
  getTokenExpiry,
  removeToken,
  clearAll,
  hasValidToken,
} from './auth';
