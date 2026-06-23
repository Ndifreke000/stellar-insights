import NetInfo from '@react-native-community/netinfo';
import { processOfflineQueue } from '@hooks/useOfflineQueue';
import { useAppStore } from '@store/appStore';

export async function setupNetworkMonitoring(): Promise<void> {
  // Fetch the initial network state immediately so the app doesn't assume it's
  // online before the first change event fires (prevents crashes on cold start
  // with no connectivity).
  const initialState = await NetInfo.fetch();
  const initiallyOnline = (initialState.isConnected && initialState.isInternetReachable) ?? false;
  useAppStore.getState().setOnlineStatus(initiallyOnline);

  NetInfo.addEventListener(state => {
    const isOnline = (state.isConnected && state.isInternetReachable) ?? false;
    useAppStore.getState().setOnlineStatus(isOnline);

    if (isOnline) {
      syncOfflineData();
    }
  });
}

async function syncOfflineData(): Promise<void> {
  await processOfflineQueue();
}
