import React from 'react';
import { Platform, RefreshControlProps } from 'react-native';
import { useQueryClient, type QueryClient } from '@tanstack/react-query';
import { useAppStore } from '@store/appStore';

export interface UsePullToRefreshResult {
  isRefreshing: boolean;
  onRefresh: () => Promise<void>;
  refreshControlProps: Partial<RefreshControlProps>;
}

export interface PullToRefreshConfig {
  queryKeyPrefix?: string;
  platformThreshold?: { ios: number; android: number };
  enabled?: boolean;
}

const PLATFORM_THRESHOLDS = {
  ios: 50,
  android: 60,
};

/**
 * Hook for managing pull-to-refresh functionality
 * Provides platform-specific optimization and query client integration
 */
export function usePullToRefresh(config?: PullToRefreshConfig): UsePullToRefreshResult {
  const [isRefreshing, setIsRefreshing] = React.useState(false);
  const queryClient = useQueryClient();
  const { isOnline } = useAppStore();

  const thresholds = config?.platformThreshold || PLATFORM_THRESHOLDS;
  const platformThreshold =
    Platform.select({
      ios: thresholds.ios / 100,
      android: thresholds.android / 100,
      default: 0.5,
    }) || 0.5;

  const onRefresh = React.useCallback(async () => {
    if (!isOnline) {
      setIsRefreshing(false);
      return;
    }

    setIsRefreshing(true);

    try {
      if (config?.queryKeyPrefix) {
        // Invalidate specific queries by prefix
        await queryClient.invalidateQueries({
          queryKey: [config.queryKeyPrefix],
        });
      } else {
        // Invalidate all queries
        await queryClient.invalidateQueries();
      }

      // Give queries time to refetch
      await new Promise(resolve => setTimeout(resolve, 300));
    } catch (error) {
      console.warn('Pull-to-refresh error:', error);
    } finally {
      setIsRefreshing(false);
    }
  }, [queryClient, isOnline, config?.queryKeyPrefix]);

  const refreshControlProps: Partial<RefreshControlProps> = {
    refreshing: isRefreshing,
    onRefresh,
    enabled: config?.enabled !== false && isOnline,
    // Platform-specific colors
    progressBackgroundColor: Platform.select({
      ios: undefined,
      android: '#FFFFFF',
      default: '#FFFFFF',
    }),
    tintColor: Platform.select({
      ios: '#999999',
      android: undefined,
      default: '#999999',
    }),
    size: Platform.select({
      ios: undefined,
      android: 'large',
      default: undefined,
    }) as RefreshControlProps['size'],
  };

  return {
    isRefreshing,
    onRefresh,
    refreshControlProps,
  };
}

/**
 * Standalone function to manually trigger refresh
 * Useful for manual refresh buttons outside of FlatList context.
 *
 * Takes the QueryClient explicitly rather than calling useQueryClient()
 * itself -- this is a plain async function meant to be invoked from event
 * handlers (e.g. a refresh button's onPress), not from render, so it can't
 * call a hook internally. Callers get their queryClient the normal way
 * (useQueryClient() in their own component) and pass it through.
 */
export async function refreshAllQueries(queryClient: QueryClient): Promise<void> {
  await queryClient.invalidateQueries();
}
