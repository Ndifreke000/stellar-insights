/**
 * Biometric Authentication Service (WebAuthn)
 *
 * Root cause of post-update failures:
 *   1. rpId / origin mismatch after domain or path changes
 *   2. Stale credential IDs stored in localStorage from a previous app version
 *   3. Authenticator internal state invalidated by OS/browser update
 *
 * Fix strategy:
 *   - Version credentials in storage; on mismatch, clear stale data and re-register
 *   - Classify WebAuthn errors precisely so the UI can offer the right recovery path
 *   - Never throw raw DOMException to callers — always return a typed result
 *   - Track app version and invalidate credentials on major version changes
 *   - Implement credential expiry for periodic refresh (90 days default)
 */

// Increment this on every breaking app update; also checked against localStorage stored version
export const CREDENTIAL_VERSION = 3;

// Credential expiry: force re-registration every 90 days
export const CREDENTIAL_EXPIRY_MS = 90 * 24 * 60 * 60 * 1000;

const STORAGE_KEY = 'biometric_credential';
const APP_VERSION_KEY = 'app_version';

export type BiometricResult<T = void> =
  | { ok: true; data: T }
  | { ok: false; error: BiometricError };

export type BiometricError =
  | 'NOT_SUPPORTED'        // WebAuthn unavailable in this browser/platform
  | 'NOT_ENROLLED'         // No credential registered for this user
  | 'STALE_CREDENTIAL'     // Credential exists but is from an old app version
  | 'USER_CANCELLED'       // User dismissed the biometric prompt
  | 'INVALID_STATE'        // Credential already registered (duplicate)
  | 'SECURITY_ERROR'       // rpId / origin mismatch — most common post-update cause
  | 'UNKNOWN';

export interface StoredCredential {
  credentialId: string;   // base64url-encoded
  userId: string;
  version: number;
  registeredAt: number;
  expiresAt?: number;     // New: credential expiry timestamp for forced refresh
  appVersion?: string;    // New: version of app when credential was registered
}

// ─── Helpers ────────────────────────────────────────────────────────────────

function getAppVersion(): string {
  // In production, this would come from package.json via build process
  // For now, use a timestamp-based version that increments with deployments
  if (typeof window !== 'undefined' && (window as any).__APP_VERSION__) {
    return (window as any).__APP_VERSION__;
  }
  return '1.0.0'; // fallback
}

function base64urlEncode(buffer: ArrayBuffer): string {
  return btoa(String.fromCharCode(...new Uint8Array(buffer)))
    .replace(/\+/g, '-')
    .replace(/\//g, '_')
    .replace(/=/g, '');
}

function base64urlDecode(str: string): Uint8Array {
  const padded = str.replace(/-/g, '+').replace(/_/g, '/');
  const binary = atob(padded);
  return Uint8Array.from(binary, c => c.charCodeAt(0));
}

function classifyError(err: unknown): BiometricError {
  if (!(err instanceof Error)) return 'UNKNOWN';
  const name = (err as DOMException).name;
  if (name === 'NotAllowedError') return 'USER_CANCELLED';
  if (name === 'InvalidStateError') return 'INVALID_STATE';
  if (name === 'SecurityError') return 'SECURITY_ERROR';
  if (name === 'NotSupportedError') return 'NOT_SUPPORTED';
  return 'UNKNOWN';
}

function getRpId(): string {
  // Use the current hostname so rpId is always consistent with the origin.
  // This is the most common source of post-update breakage when the domain changes.
  if (typeof window !== 'undefined') {
    try {
      return window.location.hostname;
    } catch {
      console.warn('[biometric] Failed to get hostname, using localhost fallback');
      return 'localhost';
    }
  }
  return 'localhost';
}

// ─── Storage ────────────────────────────────────────────────────────────────

export function loadCredential(userId: string): StoredCredential | null {
  try {
    const raw = localStorage.getItem(`${STORAGE_KEY}:${userId}`);
    if (!raw) return null;
    return JSON.parse(raw) as StoredCredential;
  } catch {
    return null;
  }
}

function saveCredential(cred: StoredCredential): void {
  localStorage.setItem(`${STORAGE_KEY}:${cred.userId}`, JSON.stringify(cred));
}

export function clearCredential(userId: string): void {
  localStorage.removeItem(`${STORAGE_KEY}:${userId}`);
}

/**
 * Returns true when a stored credential exists but was created by an older
 * app version — the primary cause of post-update biometric failures.
 * Also checks if credential has expired (90 day default).
 */
export function isStaleCredential(userId: string): boolean {
  const cred = loadCredential(userId);
  if (cred === null) return false;
  
  // Version mismatch = stale
  if (cred.version < CREDENTIAL_VERSION) return true;
  
  // Expired credential = stale
  if (cred.expiresAt && cred.expiresAt < Date.now()) return true;
  
  return false;
}

// ─── Feature detection ──────────────────────────────────────────────────────

export function isBiometricSupported(): boolean {
  return (
    typeof window !== 'undefined' &&
    typeof window.PublicKeyCredential !== 'undefined' &&
    typeof navigator.credentials !== 'undefined'
  );
}

export async function isPlatformAuthenticatorAvailable(): Promise<boolean> {
  if (!isBiometricSupported()) return false;
  try {
    return await PublicKeyCredential.isUserVerifyingPlatformAuthenticatorAvailable();
  } catch {
    return false;
  }
}

// ─── Registration ───────────────────────────────────────────────────────────

export async function registerBiometric(
  userId: string,
  displayName: string
): Promise<BiometricResult<StoredCredential>> {
  if (!isBiometricSupported()) return { ok: false, error: 'NOT_SUPPORTED' };

  // Clear any stale credential before re-registering so the authenticator
  // doesn't reject with InvalidStateError on a version bump.
  if (isStaleCredential(userId)) {
    clearCredential(userId);
  }

  const challenge = crypto.getRandomValues(new Uint8Array(32));
  const userIdBytes = new TextEncoder().encode(userId);
  const rpId = getRpId();

  const options: PublicKeyCredentialCreationOptions = {
    rp: { id: rpId, name: 'Stellar Insights' },
    user: { id: userIdBytes, name: userId, displayName },
    challenge,
    pubKeyCredParams: [
      { type: 'public-key', alg: -7 },   // ES256
      { type: 'public-key', alg: -257 },  // RS256 fallback
    ],
    authenticatorSelection: {
      authenticatorAttachment: 'platform',
      userVerification: 'required',
      residentKey: 'preferred',
    },
    timeout: 60_000,
    attestation: 'none',
  };

  try {
    const credential = await navigator.credentials.create({ publicKey: options });
    if (!credential) return { ok: false, error: 'UNKNOWN' };

    const pkCred = credential as PublicKeyCredential;
    const now = Date.now();
    const stored: StoredCredential = {
      credentialId: base64urlEncode(pkCred.rawId),
      userId,
      version: CREDENTIAL_VERSION,
      registeredAt: now,
      expiresAt: now + CREDENTIAL_EXPIRY_MS,
      appVersion: getAppVersion(),
    };

    saveCredential(stored);
    return { ok: true, data: stored };
  } catch (err) {
    return { ok: false, error: classifyError(err) };
  }
}

// ─── Authentication ──────────────────────────────────────────────────────────

export async function authenticateWithBiometric(
  userId: string
): Promise<BiometricResult<{ credentialId: string; timestamp: number }>> {
  if (!isBiometricSupported()) return { ok: false, error: 'NOT_SUPPORTED' };

  const stored = loadCredential(userId);
  if (!stored) return { ok: false, error: 'NOT_ENROLLED' };

  // Post-update: credential version mismatch means the stored credential ID
  // may no longer be valid. Signal STALE_CREDENTIAL so the UI can re-register.
  if (stored.version < CREDENTIAL_VERSION) {
    return { ok: false, error: 'STALE_CREDENTIAL' };
  }

  // Credential has expired after 90 days — force re-registration
  if (stored.expiresAt && stored.expiresAt < Date.now()) {
    clearCredential(userId);
    return { ok: false, error: 'STALE_CREDENTIAL' };
  }

  const challenge = crypto.getRandomValues(new Uint8Array(32));
  const rpId = getRpId();

  const options: PublicKeyCredentialRequestOptions = {
    rpId,
    challenge,
    allowCredentials: [
      {
        type: 'public-key',
        id: base64urlDecode(stored.credentialId),
        transports: ['internal'],
      },
    ],
    userVerification: 'required',
    timeout: 60_000,
  };

  try {
    const assertion = await navigator.credentials.get({ publicKey: options });
    if (!assertion) return { ok: false, error: 'UNKNOWN' };

    return {
      ok: true,
      data: {
        credentialId: stored.credentialId,
        timestamp: Date.now(),
      },
    };
  } catch (err) {
    const error = classifyError(err);

    // SecurityError at auth time = rpId/origin changed (e.g. domain update).
    // Treat as stale so the UI offers re-registration rather than a generic error.
    // Also log for debugging post-update issues.
    if (error === 'SECURITY_ERROR') {
      console.warn('[biometric] SecurityError during auth — rpId mismatch detected. rpId:', rpId);
      clearCredential(userId);
      return { ok: false, error: 'STALE_CREDENTIAL' };
    }

    return { ok: false, error };
  }
}
