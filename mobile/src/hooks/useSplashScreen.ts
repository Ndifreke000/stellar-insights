import React from 'react';
import { Platform } from 'react-native';
import { initializeApp } from '@services/initialization';

export type SplashStatus = 'loading' | 'ready' | 'error';

export interface UseSplashScreenResult {
  status: SplashStatus;
  error: string | null;
  isVisible: boolean;
  platformName: string;
}

export function useSplashScreen(): UseSplashScreenResult {
  const [status, setStatus] = React.useState<SplashStatus>('loading');
  const [error, setError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;

    initializeApp()
      .then(() => {
        if (!cancelled) setStatus('ready');
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : 'Initialization failed');
          setStatus('error');
        }
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return {
    status,
    error,
    isVisible: status === 'loading',
    platformName: Platform.select({ ios: 'iOS', android: 'Android', default: 'Unknown' }) ?? 'Unknown',
  };
}
