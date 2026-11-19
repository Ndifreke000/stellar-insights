import NetInfo from '@react-native-community/netinfo';
import { processOfflineQueue } from '@hooks/useOfflineQueue';
import { useAppStore } from '@store/appStore';
import { syncWatchlistToBackend } from './watchlist';
import { logger } from '@utils/logger';

export function setupNetworkMonitoring(): void {
  NetInfo.addEventListener(state => {
    const isOnline = state.isConnected && state.isInternetReachable;
    useAppStore.getState().setOnlineStatus(isOnline ?? false);

    if (isOnline) {
      logger.info('[Network]', 'Connectivity restored — flushing offline queues.');
      // Trigger sync when coming back online
      syncOfflineData();
    }
  });
}

async function syncOfflineData(): Promise<void> {
  await Promise.allSettled([
    processOfflineQueue(),
    syncWatchlistToBackend(),
  ]);
}
