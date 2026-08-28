import React from 'react';
import { StatusBar } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { NavigationContainer, LinkingOptions, getStateFromPath } from '@react-navigation/native';
import NetInfo from '@react-native-community/netinfo';
import { RootNavigator } from './navigation/RootNavigator';
import type { RootStackParamList } from './navigation/RootNavigator';
import { useAppStore } from './store/appStore';
import { useAuthStore } from './store/authStore';
import { initializeApp } from './services/initialization';
import { hasValidToken } from './services/tokenStorage';
import { processOfflineQueue } from './hooks/useOfflineQueue';
import { NetworkStatusIndicator } from './components/NetworkStatusIndicator';
import { OfflineCachingIndicator } from './components/OfflineCaching';
import { OfflineBanner } from './components/OfflineBanner';
import { SyncStatusBanner } from './components/SyncStatusBanner';

import { NetworkProvider, getCurrentNetwork } from '@config/network';

const linking: LinkingOptions<RootStackParamList> = {
  prefixes: ['stellar-insights://'],
  config: {
    screens: {
      Main: {
        screens: {
          Dashboard: 'dashboard',
          Anchors: {
            screens: {
              AnchorsList: 'anchors',
              AnchorDetail: 'anchors/:anchorId',
            },
          },
          Corridors: {
            screens: {
              CorridorsList: 'corridors',
              CorridorDetail: 'corridors/:corridorId',
            },
          },
          NFCSupport: 'nfc-support',
          BluetoothSupport: 'bluetooth-support',
          BeaconSupport: 'beacon-support',
          AirDropIntegration: 'airdrop',
          ShortcutsSupport: 'shortcuts',
          AppIntents: 'app-intents',
          QuickActions: 'quick-actions',
          ShareExtension: 'share-extension',
          ActionExtension: 'action-extension',
          ForceTouch: 'force-touch',
          HandoffSupport: 'handoff-support',
          Geofencing: 'geofencing',
          BackgroundSync: 'background-sync',
          GestureControls: 'gesture-controls',
          ContactsIntegration: 'contacts',
          MapsIntegration: 'maps',
          WidgetSupport: 'widget-support',
          ThreeDTouchSupport: '3d-touch',
          ShakeToRefresh: 'shake-to-refresh',
          AppClips: 'app-clips',
          LiveActivities: 'live-activities',
          WatchApp: 'watch-app',
          WearOSApp: 'wear-os-app',
          VideoPlayer: 'video-player',
          AudioRecording: 'audio-recording',
        },
      },
      Auth: {
        screens: {
          Login: 'login',
        },
      },
      Error: 'error',
    },
  },
  getStateFromPath(path, options) {
    const matchAnchor = path.match(/anchors\/([^\/]+)/);
    if (matchAnchor) {
      const anchorId = matchAnchor[1];
      if (!/^[A-F0-9]{64}$/i.test(anchorId)) {
        return {
          routes: [
            {
              name: 'Error',
              params: {
                message: 'Invalid Anchor ID. Deep links must contain a 64-character hexadecimal ID.',
              },
            },
          ],
        };
      }
    }

    const matchCorridor = path.match(/corridors\/([^\/]+)/);
    if (matchCorridor) {
      const corridorId = matchCorridor[1];
      if (!/^[A-F0-9]{64}$/i.test(corridorId)) {
        return {
          routes: [
            {
              name: 'Error',
              params: {
                message: 'Invalid Corridor ID. Deep links must contain a 64-character hexadecimal ID.',
              },
            },
          ],
        };
      }
    }

    return getStateFromPath(path, options);
  },
};

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 2,
      staleTime: 5 * 60 * 1000, // 5 minutes
      gcTime: 10 * 60 * 1000, // 10 minutes
    },
  },
});

function App(): React.JSX.Element {
  const { theme, isOnline } = useAppStore();
  const isDark = theme === 'dark';

  React.useEffect(() => {
    void (async () => {
      // Resolve initial network state before any API calls to prevent crashes on offline startup.
      const netState = await NetInfo.fetch();
      const initiallyOnline = (netState.isConnected && netState.isInternetReachable) ?? false;
      useAppStore.getState().setOnlineStatus(initiallyOnline);

      // Load network settings before initializing App
      const initialNetConfig = await getCurrentNetwork();
      useAppStore.getState().setNetwork(initialNetConfig.id);

      await initializeApp();
      // Decide the initial route from securely stored token presence/expiry.
      try {
        if (await hasValidToken()) {
          useAuthStore.setState({ isAuthenticated: true });
        }
      } finally {
        useAuthStore.setState({ isLoading: false });
      }
    })();
  }, []);

  React.useEffect(() => {
    if (isOnline) {
      processOfflineQueue();
    }
  }, [isOnline]);

  return (
    <GestureHandlerRootView style={{ flex: 1 }}>
      <NetworkProvider>
        <SafeAreaProvider>
          <QueryClientProvider client={queryClient}>
            <NavigationContainer linking={linking}>
              <StatusBar barStyle={isDark ? 'light-content' : 'dark-content'} />
              <OfflineBanner />
              <SyncStatusBanner />
              <NetworkStatusIndicator />
              <OfflineCachingIndicator showCacheSize={true} />
              <RootNavigator />
            </NavigationContainer>
            {/* Widget, 3D Touch, and Shake-to-Refresh are registered via MainNavigator */}
          </QueryClientProvider>
        </SafeAreaProvider>
      </NetworkProvider>
    </GestureHandlerRootView>
  );
}

export default App;
