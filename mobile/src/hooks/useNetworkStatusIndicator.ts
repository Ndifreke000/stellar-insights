import React from 'react';
import { Platform } from 'react-native';
import { useAppStore } from '@store/appStore';

export type NetworkStatus = 'online' | 'offline' | 'syncing';

export interface NetworkStatusIndicatorState {
  status: NetworkStatus;
  message: string;
  isVisible: boolean;
  isOnline: boolean;
  isSyncing: boolean;
  platformOffset: number;
}

export interface UseNetworkStatusIndicatorResult extends NetworkStatusIndicatorState {
  dismiss: () => void;
  show: () => void;
}

export function useNetworkStatusIndicator(): UseNetworkStatusIndicatorResult {
  const { isOnline, isSyncing } = useAppStore();
  const [isDismissed, setIsDismissed] = React.useState(false);

  const status: NetworkStatus = isSyncing ? 'syncing' : isOnline ? 'online' : 'offline';
  const message = status === 'syncing' ? 'Syncing offline changes' : status === 'online' ? 'Back online' : 'Offline mode active';
  const platformOffset = Platform.select({ ios: 8, android: 0, default: 0 }) ?? 0;

  React.useEffect(() => {
    setIsDismissed(false);
  }, [status]);

  return {
    status,
    message,
    isVisible: !isDismissed && (!isOnline || isSyncing),
    isOnline,
    isSyncing,
    platformOffset,
    dismiss: () => setIsDismissed(true),
    show: () => setIsDismissed(false),
  };
}
