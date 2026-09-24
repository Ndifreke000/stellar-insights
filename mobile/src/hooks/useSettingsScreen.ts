import React from 'react';
import { Platform } from 'react-native';
import { StellarNetwork } from '../types';
import { useAppStore } from '@store/appStore';

export interface SettingsScreenState {
  theme: 'light' | 'dark';
  network: StellarNetwork;
  isOnline: boolean;
  isSyncing: boolean;
  pendingMessage?: string;
  error?: string;
  platformLabel: string;
}

export interface UseSettingsScreenResult extends SettingsScreenState {
  toggleTheme: () => void;
  toggleNetwork: () => void;
  clearMessage: () => void;
}

export function useSettingsScreen(): UseSettingsScreenResult {
  const { theme, network, isOnline, isSyncing, setTheme, setNetwork } = useAppStore();
  const [pendingMessage, setPendingMessage] = React.useState<string | undefined>();
  const [error, setError] = React.useState<string | undefined>();

  const platformLabel =
    Platform.select({ ios: 'iOS', android: 'Android', default: 'Mobile' }) ?? 'Mobile';

  const clearMessage = React.useCallback(() => {
    setPendingMessage(undefined);
    setError(undefined);
  }, []);

  const toggleTheme = React.useCallback(() => {
    try {
      setTheme(theme === 'light' ? 'dark' : 'light');
      setPendingMessage(
        isOnline ? 'Theme updated' : 'Theme saved locally and will apply while offline'
      );
      setError(undefined);
    } catch {
      setError('Unable to update theme setting');
    }
  }, [isOnline, setTheme, theme]);

  const toggleNetwork = React.useCallback(() => {
    try {
      const nextNetwork: StellarNetwork = network === 'testnet' ? 'mainnet' : 'testnet';
      setNetwork(nextNetwork);
      setPendingMessage(
        isOnline
          ? `Network changed to ${nextNetwork}`
          : `Network changed to ${nextNetwork} while offline`
      );
      setError(undefined);
    } catch {
      setError('Unable to update network setting');
    }
  }, [isOnline, network, setNetwork]);

  return {
    theme,
    network,
    isOnline,
    isSyncing,
    pendingMessage,
    error,
    platformLabel,
    toggleTheme,
    toggleNetwork,
    clearMessage,
  };
}
