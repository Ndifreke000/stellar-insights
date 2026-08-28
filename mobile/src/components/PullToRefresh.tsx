import React from 'react';
import {
  ActivityIndicator,
  FlatList,
  Platform,
  RefreshControl,
  StyleSheet,
  Text,
  View,
  ViewStyle,
} from 'react-native';
import { usePullToRefresh, PullToRefreshConfig } from '@hooks/usePullToRefresh';

export interface PullToRefreshProps {
  children?: React.ReactNode;
  onRefresh?: () => Promise<void>;
  isLoading?: boolean;
  config?: PullToRefreshConfig;
  contentContainerStyle?: ViewStyle;
  showLoadingIndicator?: boolean;
  testID?: string;
}

/**
 * Pull-to-Refresh component that wraps FlatList
 * Provides platform-optimized refresh control with error handling
 */
export const PullToRefresh: React.FC<PullToRefreshProps> = ({
  children,
  onRefresh: customOnRefresh,
  isLoading = false,
  config,
  contentContainerStyle,
  showLoadingIndicator = true,
  testID,
}) => {
  const { isRefreshing, onRefresh, refreshControlProps } = usePullToRefresh(config);

  const handleRefresh = React.useCallback(async () => {
    try {
      if (customOnRefresh) {
        await customOnRefresh();
      }
      await onRefresh();
    } catch (error) {
      console.warn('Refresh failed:', error);
    }
  }, [onRefresh, customOnRefresh]);

  return (
    <View
      style={styles.container}
      accessibilityLabel={testID || 'Pull to refresh container'}
      testID={testID}>
      {isRefreshing && showLoadingIndicator && (
        <View style={styles.loadingOverlay} accessibilityLiveRegion="polite">
          <ActivityIndicator
            size={Platform.select({ ios: 'large', android: 'large', default: 'large' })}
            color="#0066CC"
            accessibilityLabel="Refreshing data"
          />
        </View>
      )}

      <View style={[styles.content, contentContainerStyle]}>{children}</View>
    </View>
  );
};

/**
 * PullToRefreshList component for FlatList with built-in pull-to-refresh
 */
export interface PullToRefreshListProps<T> {
  data: T[];
  renderItem: (item: { item: T; index: number }) => React.ReactElement | null;
  keyExtractor: (item: T, index: number) => string;
  onRefresh?: () => Promise<void>;
  isLoading?: boolean;
  config?: PullToRefreshConfig;
  contentContainerStyle?: ViewStyle;
  ListHeaderComponent?: React.ComponentType<any> | React.ReactElement | null;
  ListFooterComponent?: React.ComponentType<any> | React.ReactElement | null;
  onEndReached?: () => void;
  onEndReachedThreshold?: number;
  testID?: string;
}

export const PullToRefreshList = React.forwardRef<FlatList, PullToRefreshListProps<any>>(
  (
    {
      data,
      renderItem,
      keyExtractor,
      onRefresh: customOnRefresh,
      isLoading = false,
      config,
      contentContainerStyle,
      ListHeaderComponent,
      ListFooterComponent,
      onEndReached,
      onEndReachedThreshold = 0.8,
      testID,
    },
    ref
  ) => {
    const { isRefreshing, onRefresh, refreshControlProps } = usePullToRefresh(config);

    const handleRefresh = React.useCallback(async () => {
      try {
        if (customOnRefresh) {
          await customOnRefresh();
        }
        await onRefresh();
      } catch (error) {
        console.warn('List refresh failed:', error);
      }
    }, [onRefresh, customOnRefresh]);

    return (
      <FlatList
        ref={ref}
        data={data}
        renderItem={renderItem}
        keyExtractor={keyExtractor}
        refreshControl={
          <RefreshControl {...(refreshControlProps as any)} onRefresh={handleRefresh} />
        }
        ListHeaderComponent={ListHeaderComponent}
        ListFooterComponent={ListFooterComponent}
        onEndReached={onEndReached}
        onEndReachedThreshold={onEndReachedThreshold}
        contentContainerStyle={contentContainerStyle}
        accessibilityLabel={testID || 'Pull to refresh list'}
        testID={testID}
      />
    );
  }
);

PullToRefreshList.displayName = 'PullToRefreshList';

const styles = StyleSheet.create({
  container: {
    flex: 1,
    position: 'relative',
  },
  content: {
    flex: 1,
  },
  loadingOverlay: {
    position: 'absolute',
    top: 0,
    left: 0,
    right: 0,
    bottom: 0,
    justifyContent: 'center',
    alignItems: 'center',
    zIndex: 100,
    backgroundColor: 'rgba(0, 0, 0, 0.1)',
  },
});
