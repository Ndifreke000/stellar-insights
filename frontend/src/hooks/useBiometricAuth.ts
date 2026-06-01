'use client';

import { useState, useCallback, useEffect } from 'react';
import {
  isBiometricSupported,
  isPlatformAuthenticatorAvailable,
  CREDENTIAL_VERSION,
  loadCredential,
  registerBiometric,
  authenticateWithBiometric,
  clearCredential,
  type BiometricError,
  type StoredCredential,
} from '@/services/biometricAuth';

export type BiometricState =
  | 'idle'
  | 'checking'
  | 'authenticating'
  | 'registering'
  | 'success'
  | 'error';

export interface UseBiometricAuthReturn {
  state: BiometricState;
  error: BiometricError | null;
  isSupported: boolean;
  isEnrolled: boolean;
  isStale: boolean;
  credential: StoredCredential | null;
  authenticate: (userId: string) => Promise<boolean>;
  register: (userId: string, displayName: string) => Promise<boolean>;
  reset: (userId: string) => void;
}

export function useBiometricAuth(userId: string): UseBiometricAuthReturn {
  const [state, setState] = useState<BiometricState>('checking');
  const [error, setError] = useState<BiometricError | null>(null);
  const [isSupported, setIsSupported] = useState(false);
  const [credential, setCredential] = useState<StoredCredential | null>(null);

  useEffect(() => {
    let cancelled = false;
    async function check() {
      const supported =
        isBiometricSupported() && (await isPlatformAuthenticatorAvailable());
      if (cancelled) return;
      setIsSupported(supported);
      setCredential(loadCredential(userId));
      setState('idle');
    }
    check();
    return () => { cancelled = true; };
  }, [userId]);

  const authenticate = useCallback(
    async (uid: string): Promise<boolean> => {
      setState('authenticating');
      setError(null);
      const result = await authenticateWithBiometric(uid);
      if (result.ok) {
        setState('success');
        return true;
      }
      setError(result.error);
      setState('error');
      // Refresh credential state so UI reflects stale/missing status
      setCredential(loadCredential(uid));
      return false;
    },
    []
  );

  const register = useCallback(
    async (uid: string, displayName: string): Promise<boolean> => {
      setState('registering');
      setError(null);
      const result = await registerBiometric(uid, displayName);
      if (result.ok) {
        setCredential(result.data);
        setState('success');
        return true;
      }
      setError(result.error);
      setState('error');
      return false;
    },
    []
  );

  const reset = useCallback((uid: string) => {
    clearCredential(uid);
    setCredential(null);
    setError(null);
    setState('idle');
  }, []);

  const isStale =
    credential !== null &&
    (credential.version < CREDENTIAL_VERSION ||
      (credential.expiresAt !== undefined && credential.expiresAt < Date.now()));

  return {
    state,
    error,
    isSupported,
    isEnrolled: credential !== null,
    isStale,
    credential,
    authenticate,
    register,
    reset,
  };
}
