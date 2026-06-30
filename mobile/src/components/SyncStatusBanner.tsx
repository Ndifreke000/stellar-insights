import React from 'react';
import { ActivityIndicator, Platform, Pressable, StyleSheet, Text, View } from 'react-native';
import { useAppStore } from '@store/appStore';
import { useOfflineQueue } from '@hooks/useOfflineQueue';

export interface SyncStatusBannerProps {
  /** Called when the user taps "Retry" on a sync failure. */
  onRetry?: () => void;
  testID?: string;
}

/**
 * Surfaces the current sync state to the user.
 *
 * Visible states:
 *   • Syncing   – spinner + "Syncing…" while the offline queue is being processed
 *   • Pending   – badge showing the number of queued requests awaiting connectivity
 *   • Failed    – warning banner with a count of failed items and a Retry button
 *
 * The component renders nothing when everything is up-to-date and online.
 */
export const SyncStatusBanner: React.FC<SyncStatusBannerProps> = ({
  onRetry,
  testID,
}) => {
  const { isSyncing, isOnline } = useAppStore();
  const { items, isProcessing, retryFailed } = useOfflineQueue();

  const pendingCount = items.filter(i => i.status === 'pending').length;
  const failedCount = items.filter(i => i.status === 'failed').length;

  const handleRetry = () => {
    void retryFailed();
    onRetry?.();
  };

  // Syncing in progress
  if (isSyncing || isProcessing) {
    return (
      <View
        style={[styles.banner, styles.syncing]}
        testID={testID}
        accessibilityRole="status"
        accessibilityLabel="Syncing data in the background"
        accessibilityLiveRegion="polite"
      >
        <ActivityIndicator
          size="small"
          color="#1d4ed8"
          accessibilityElementsHidden
        />
        <Text style={styles.syncingText}>Syncing…</Text>
      </View>
    );
  }

  // Failed items that need manual attention
  if (failedCount > 0) {
    return (
      <View
        style={[styles.banner, styles.failed]}
        testID={testID}
        accessibilityRole="alert"
        accessibilityLabel={`${failedCount} sync ${failedCount === 1 ? 'request' : 'requests'} failed. Tap retry to try again.`}
      >
        <Text style={styles.failedIcon} accessibilityElementsHidden>
          ⚠️
        </Text>
        <Text style={styles.failedText}>
          {failedCount} sync {failedCount === 1 ? 'request' : 'requests'} failed
        </Text>
        <Pressable
          style={styles.retryButton}
          onPress={handleRetry}
          accessibilityRole="button"
          accessibilityLabel="Retry failed sync requests"
        >
          <Text style={styles.retryText}>Retry</Text>
        </Pressable>
      </View>
    );
  }

  // Pending items waiting for connectivity
  if (!isOnline && pendingCount > 0) {
    return (
      <View
        style={[styles.banner, styles.pending]}
        testID={testID}
        accessibilityRole="status"
        accessibilityLabel={`${pendingCount} ${pendingCount === 1 ? 'update' : 'updates'} queued, will sync when online`}
        accessibilityLiveRegion="polite"
      >
        <Text style={styles.pendingIcon} accessibilityElementsHidden>
          🕐
        </Text>
        <Text style={styles.pendingText}>
          {pendingCount} {pendingCount === 1 ? 'update' : 'updates'} queued
          {' — '}will sync when online
        </Text>
      </View>
    );
  }

  return null;
};

const styles = StyleSheet.create({
  banner: {
    flexDirection: 'row',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: Platform.select({ ios: 10, android: 8, default: 8 }),
    gap: 8,
  },
  // ── Syncing ──
  syncing: {
    backgroundColor: '#dbeafe',
  },
  syncingText: {
    fontSize: 13,
    fontWeight: '600',
    color: '#1d4ed8',
  },
  // ── Failed ──
  failed: {
    backgroundColor: '#fee2e2',
  },
  failedIcon: {
    fontSize: 14,
  },
  failedText: {
    flex: 1,
    fontSize: 13,
    fontWeight: '600',
    color: '#991b1b',
  },
  retryButton: {
    minHeight: 36,
    justifyContent: 'center',
    paddingHorizontal: 12,
    backgroundColor: '#dc2626',
    borderRadius: 6,
  },
  retryText: {
    fontSize: 12,
    fontWeight: '700',
    color: '#ffffff',
  },
  // ── Pending ──
  pending: {
    backgroundColor: '#fef9c3',
  },
  pendingIcon: {
    fontSize: 14,
  },
  pendingText: {
    flex: 1,
    fontSize: 13,
    fontWeight: '500',
    color: '#854d0e',
  },
});
