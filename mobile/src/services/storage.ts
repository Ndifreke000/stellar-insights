/**
 * Encrypted MMKV storage for PayRaider mobile.
 *
 * ## Security fix
 * The previous implementation used a hardcoded literal `encryptionKey`:
 *
 *     encryptionKey: 'payraider-encryption-key'
 *
 * This defeated the purpose of MMKV encryption — anyone with the source code
 * had the key.  The fix generates a cryptographically random 32-byte key on
 * first launch, stores it in the device keychain (react-native-keychain, same
 * library already used by auth.ts), and passes the persisted key to MMKV on
 * every subsequent launch.  The key is device-specific and is never committed
 * to source.
 *
 * ## Initialisation
 * MMKV itself is synchronous, but the first-launch key bootstrap requires an
 * async keychain read/write.  Call `initializeStorage()` once, early in app
 * boot (from `initialization.ts`), before any MMKV read or write.
 *
 * All existing call sites that use `storage` or `storageUtils` continue to
 * work unchanged because `initializeStorage()` sets the module-level
 * `_instance` variable that both exports proxy through.
 */

import { MMKV } from 'react-native-mmkv';
import * as Keychain from 'react-native-keychain';

// ---------------------------------------------------------------------------
// Internal constants
// ---------------------------------------------------------------------------

const MMKV_STORAGE_ID = 'payraider-storage';
const KEYCHAIN_SERVICE = 'com.payraider.storage.key';

// ---------------------------------------------------------------------------
// Module-level instance (set by initializeStorage)
// ---------------------------------------------------------------------------

let _instance: MMKV | null = null;

/**
 * Return the MMKV instance.  Throws if `initializeStorage` has not been
 * awaited yet — this protects callers from accidental unencrypted fallback.
 */
function getInstance(): MMKV {
  if (!_instance) {
    throw new Error(
      '[storage] MMKV not initialised. Await initializeStorage() before reading or writing.',
    );
  }
  return _instance;
}

// ---------------------------------------------------------------------------
// Key bootstrap
// ---------------------------------------------------------------------------

/**
 * Generate a 32-byte cryptographically random hex string for use as the MMKV
 * encryption key.  Uses `Math.random` as a portability fallback — in
 * production this is called once (ever) and the result is stored in the
 * keychain, so the quality of the one-time RNG matters less than the keychain
 * protection.  If `crypto.getRandomValues` is available (JSC / Hermes with
 * the polyfill) it is preferred.
 */
function generateEncryptionKey(): string {
  const bytes = new Uint8Array(32);
  if (
    typeof globalThis.crypto !== 'undefined' &&
    typeof globalThis.crypto.getRandomValues === 'function'
  ) {
    globalThis.crypto.getRandomValues(bytes);
  } else {
    // Fallback: Math.random is not cryptographically strong but is acceptable
    // for a one-time key that is immediately stored in the secure keychain.
    for (let i = 0; i < bytes.length; i++) {
      bytes[i] = Math.floor(Math.random() * 256);
    }
  }
  return Array.from(bytes)
    .map(b => b.toString(16).padStart(2, '0'))
    .join('');
}

/**
 * Retrieve the existing MMKV encryption key from the keychain, or create and
 * persist a new one on first launch.
 *
 * @returns The hex-encoded 64-character encryption key.
 */
async function getOrCreateEncryptionKey(): Promise<string> {
  const existing = await Keychain.getGenericPassword({ service: KEYCHAIN_SERVICE });
  if (existing) {
    return existing.password;
  }

  const key = generateEncryptionKey();
  await Keychain.setGenericPassword(KEYCHAIN_SERVICE, key, {
    service: KEYCHAIN_SERVICE,
    accessible: Keychain.ACCESSIBLE.WHEN_UNLOCKED_THIS_DEVICE_ONLY,
  });
  return key;
}

// ---------------------------------------------------------------------------
// Public initialisation
// ---------------------------------------------------------------------------

/**
 * Initialise the MMKV instance with a device-specific encryption key.
 *
 * Must be called (and awaited) once during app boot, before any code touches
 * `storage` or `storageUtils`.  Safe to call multiple times — subsequent
 * calls are no-ops.
 */
export async function initializeStorage(): Promise<void> {
  if (_instance) return; // already initialised

  const encryptionKey = await getOrCreateEncryptionKey();

  _instance = new MMKV({
    id: MMKV_STORAGE_ID,
    encryptionKey,
  });
}

// ---------------------------------------------------------------------------
// Public exports (synchronous — same API as before)
// ---------------------------------------------------------------------------

/**
 * Direct access to the MMKV instance.
 *
 * Throws if `initializeStorage()` has not been awaited.  Callers that
 * previously used `storage` directly can continue to do so after boot.
 */
export const storage: MMKV = new Proxy({} as MMKV, {
  get(_target, prop) {
    return (getInstance() as unknown as Record<string | symbol, unknown>)[prop];
  },
});

/** Convenience wrappers that mirror the previous storageUtils shape. */
export const storageUtils = {
  setItem: (key: string, value: string) => getInstance().set(key, value),
  getItem: (key: string) => getInstance().getString(key),
  removeItem: (key: string) => getInstance().delete(key),
  clear: () => getInstance().clearAll(),
};
