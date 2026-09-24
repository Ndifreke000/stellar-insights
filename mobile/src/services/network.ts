import NetInfo from '@react-native-community/netinfo';
import { processOfflineQueue } from '@hooks/useOfflineQueue';
import { useAppStore } from '@store/appStore';

export function setupNetworkMonitoring(): void {
  NetInfo.addEventListener(state => {
    const isOnline = state.isConnected && state.isInternetReachable;
    useAppStore.getState().setOnlineStatus(isOnline ?? false);

    if (isOnline) {
      // Trigger sync when coming back online
      syncOfflineData();
    }
  });
}

async function syncOfflineData(): Promise<void> {
  await processOfflineQueue();
}
