import React from 'react';
import { ActivityIndicator, Platform, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useOfflineQueue } from '@hooks/useOfflineQueue';
import { useAppStore } from '@store/appStore';

export const OfflineQueue: React.FC = () => {
  const { items, isProcessing, error, clear, processQueue, retryFailed, remove } = useOfflineQueue();
  const isOnline = useAppStore(state => state.isOnline);
  const failedCount = items.filter(item => item.status === 'failed').length;

  return (
    <ScrollView
      style={styles.container}
      contentContainerStyle={styles.content}
      accessibilityLabel="Offline queue screen"
    >
      <View style={styles.header}>
        <Text style={styles.title}>Offline Queue</Text>
        <Text style={styles.subtitle} accessibilityLabel={`Network status ${isOnline ? 'online' : 'offline'}`}>
          {isOnline ? 'Online and ready to sync' : 'Offline mode active'}
        </Text>
      </View>

      <View style={styles.summaryCard} accessible accessibilityLabel={`${items.length} queued offline requests`}>
        <Text style={styles.summaryValue}>{items.length}</Text>
        <Text style={styles.summaryLabel}>Queued requests</Text>
      </View>

      {error ? (
        <View style={styles.errorCard} accessibilityRole="alert" accessibilityLabel={`Offline queue error ${error}`}>
          <Text style={styles.errorText}>{error}</Text>
        </View>
      ) : null}

      <View style={styles.actions}>
        <Pressable
          style={[styles.actionButton, (!isOnline || isProcessing) && styles.disabledButton]}
          onPress={processQueue}
          disabled={!isOnline || isProcessing}
          accessibilityRole="button"
          accessibilityLabel="Sync offline queue"
        >
          {isProcessing ? <ActivityIndicator color="#fff" /> : <Text style={styles.actionText}>Sync now</Text>}
        </Pressable>

        <Pressable
          style={[styles.secondaryButton, failedCount === 0 && styles.disabledButton]}
          onPress={retryFailed}
          disabled={failedCount === 0}
          accessibilityRole="button"
          accessibilityLabel="Retry failed offline requests"
        >
          <Text style={styles.secondaryText}>Retry failed</Text>
        </Pressable>

        <Pressable
          style={[styles.secondaryButton, items.length === 0 && styles.disabledButton]}
          onPress={clear}
          disabled={items.length === 0}
          accessibilityRole="button"
          accessibilityLabel="Clear offline queue"
        >
          <Text style={styles.secondaryText}>Clear</Text>
        </Pressable>
      </View>

      {items.length === 0 ? (
        <View style={styles.emptyCard} accessibilityLabel="No offline requests queued">
          <Text style={styles.emptyTitle}>All caught up</Text>
          <Text style={styles.emptyText}>Requests made offline will appear here until they sync.</Text>
        </View>
      ) : (
        items.map(item => (
          <View key={item.id} style={styles.queueItem} accessible accessibilityLabel={`${item.method} request to ${item.url} is ${item.status}`}>
            <View style={styles.queueItemHeader}>
              <Text style={styles.method}>{item.method}</Text>
              <Text style={styles.status}>{item.status}</Text>
            </View>
            <Text style={styles.url}>{item.url}</Text>
            <Text style={styles.meta}>Retries: {item.retryCount}</Text>
            {item.lastError ? <Text style={styles.itemError}>{item.lastError}</Text> : null}
            <Pressable
              style={styles.removeButton}
              onPress={() => remove(item.id)}
              accessibilityRole="button"
              accessibilityLabel={`Remove queued request ${item.id}`}
            >
              <Text style={styles.removeText}>Remove</Text>
            </Pressable>
          </View>
        ))
      )}
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  content: {
    paddingBottom: Platform.select({ ios: 32, android: 24, default: 24 }),
  },
  header: {
    padding: 20,
    backgroundColor: '#fff',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  subtitle: {
    color: '#666',
    marginTop: 6,
  },
  summaryCard: {
    backgroundColor: '#fff',
    margin: 16,
    padding: 20,
    borderRadius: Platform.select({ ios: 14, android: 10, default: 12 }),
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: Platform.OS === 'ios' ? 0.1 : 0,
    shadowRadius: 4,
    elevation: 3,
  },
  summaryValue: {
    fontSize: 36,
    fontWeight: 'bold',
  },
  summaryLabel: {
    color: '#666',
    marginTop: 4,
  },
  errorCard: {
    backgroundColor: '#fee2e2',
    marginHorizontal: 16,
    marginBottom: 16,
    padding: 16,
    borderRadius: 12,
  },
  errorText: {
    color: '#991b1b',
  },
  actions: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 12,
    marginHorizontal: 16,
    marginBottom: 16,
  },
  actionButton: {
    backgroundColor: '#2563eb',
    borderRadius: 10,
    minHeight: 44,
    paddingHorizontal: 16,
    alignItems: 'center',
    justifyContent: 'center',
  },
  secondaryButton: {
    backgroundColor: '#fff',
    borderRadius: 10,
    minHeight: 44,
    paddingHorizontal: 16,
    alignItems: 'center',
    justifyContent: 'center',
    borderWidth: 1,
    borderColor: '#d1d5db',
  },
  disabledButton: {
    opacity: 0.5,
  },
  actionText: {
    color: '#fff',
    fontWeight: '600',
  },
  secondaryText: {
    color: '#111827',
    fontWeight: '600',
  },
  emptyCard: {
    backgroundColor: '#fff',
    margin: 16,
    padding: 20,
    borderRadius: 12,
    alignItems: 'center',
  },
  emptyTitle: {
    fontSize: 18,
    fontWeight: '700',
  },
  emptyText: {
    color: '#666',
    marginTop: 8,
    textAlign: 'center',
  },
  queueItem: {
    backgroundColor: '#fff',
    marginHorizontal: 16,
    marginBottom: 12,
    padding: 16,
    borderRadius: 12,
    elevation: 2,
  },
  queueItemHeader: {
    flexDirection: 'row',
    justifyContent: 'space-between',
    marginBottom: 8,
  },
  method: {
    fontWeight: '700',
    color: '#2563eb',
  },
  status: {
    color: '#666',
    textTransform: 'capitalize',
  },
  url: {
    fontWeight: '600',
  },
  meta: {
    color: '#666',
    marginTop: 6,
  },
  itemError: {
    color: '#991b1b',
    marginTop: 8,
  },
  removeButton: {
    alignSelf: 'flex-start',
    marginTop: 12,
    minHeight: 44,
    justifyContent: 'center',
  },
  removeText: {
    color: '#dc2626',
    fontWeight: '600',
  },
});
