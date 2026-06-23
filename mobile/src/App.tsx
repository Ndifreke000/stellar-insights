import React from 'react';
import { StatusBar } from 'react-native';
import { SafeAreaProvider } from 'react-native-safe-area-context';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GestureHandlerRootView } from 'react-native-gesture-handler';
import { NavigationContainer, LinkingOptions } from '@react-navigation/native';
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
    },
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
      <SafeAreaProvider>
        <QueryClientProvider client={queryClient}>
          <NavigationContainer linking={linking}>
            <StatusBar barStyle={isDark ? 'light-content' : 'dark-content'} />
            <OfflineBanner />
            <NetworkStatusIndicator />
            <OfflineCachingIndicator showCacheSize={true} />
            <RootNavigator />
          </NavigationContainer>
          {/* Widget, 3D Touch, and Shake-to-Refresh are registered via MainNavigator */}
        </QueryClientProvider>
      </SafeAreaProvider>
    </GestureHandlerRootView>
  );
}

export default App;
