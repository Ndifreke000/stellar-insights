import { useCallback, useEffect, useState } from 'react';

import {
  getToken,
  removeToken as removeStoredToken,
  saveToken as saveStoredToken,
} from '@services/auth';

/** Public API returned by {@link useSecureTokenStorage}. */
export interface UseSecureTokenStorageReturn {
  token: string | null;
  isLoading: boolean;
  error: string | null;
  saveToken: (token: string, expiresAt?: number) => Promise<void>;
  removeToken: () => Promise<void>;
  refresh: () => Promise<void>;
}

/**
 * React hook over the secure token storage service. Loads the stored token on
 * mount and exposes save/remove operations. Storage failures are surfaced via
 * the `error` field rather than thrown, so consumers never need a try/catch.
 *
 * @returns {@link UseSecureTokenStorageReturn}
 */
export function useSecureTokenStorage(): UseSecureTokenStorageReturn {
  const [token, setToken] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      const stored = await getToken();
      setToken(stored);
    } catch {
      setError('Failed to read secure storage');
      setToken(null);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const saveToken = useCallback(
    async (value: string, expiresAt?: number) => {
      setError(null);
      try {
        await saveStoredToken(value, expiresAt);
        setToken(value);
      } catch {
        setError('Failed to save secure storage');
      }
    },
    [],
  );

  const removeToken = useCallback(async () => {
    setError(null);
    try {
      await removeStoredToken();
      setToken(null);
    } catch {
      setError('Failed to clear secure storage');
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return { token, isLoading, error, saveToken, removeToken, refresh };
}
