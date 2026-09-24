import { useCallback, useEffect, useState } from 'react';

import {
  authenticate as authenticateBiometric,
  getBiometricType,
  isBiometricAvailable,
  type BiometricType,
} from '@services/biometricService';

/** Public API returned by {@link useBiometricAuthentication}. */
export interface UseBiometricAuthenticationReturn {
  isAvailable: boolean;
  biometricType: BiometricType;
  authenticate: (reason: string) => Promise<boolean>;
  isLoading: boolean;
  error: string | null;
}

/**
 * React hook over the biometric service. Probes availability and biometric
 * type on mount and exposes an `authenticate` action. Hardware/runtime failures
 * are surfaced via `error` and resolve `authenticate` to `false` rather than
 * throwing, so callers can cleanly fall back to credential entry.
 *
 * @returns {@link UseBiometricAuthenticationReturn}
 */
export function useBiometricAuthentication(): UseBiometricAuthenticationReturn {
  const [isAvailable, setIsAvailable] = useState(false);
  const [biometricType, setBiometricType] = useState<BiometricType>('None');
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    void (async () => {
      try {
        const [available, type] = await Promise.all([
          isBiometricAvailable(),
          getBiometricType(),
        ]);
        if (!active) {
          return;
        }
        setIsAvailable(available);
        setBiometricType(type);
      } catch {
        if (active) {
          setError('Unable to check biometric availability');
        }
      } finally {
        if (active) {
          setIsLoading(false);
        }
      }
    })();
    return () => {
      active = false;
    };
  }, []);

  const authenticate = useCallback(async (reason: string): Promise<boolean> => {
    setError(null);
    try {
      return await authenticateBiometric(reason);
    } catch {
      setError('Biometric authentication is unavailable right now');
      return false;
    }
  }, []);

  return { isAvailable, biometricType, authenticate, isLoading, error };
}
