import React from 'react';
import {
  ActivityIndicator,
  Alert,
  Button,
  FlatList,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useBackgroundSync } from '@features/background_sync/useBackgroundSync';

export const BackgroundSyncComponent: React.FC = () => {
  const { loading, error, supported, tasks, queueTask, syncNow } = useBackgroundSync();

  React.useEffect(() => {
    if (error) {
      Alert.alert('Background Sync Error', error, [{ text: 'OK' }], { cancelable: true });
    }
  }, [error]);

  const handleQueueTask = () => {
    queueTask({
      type: 'sync_data',
      data: { timestamp: Date.now(), action: 'update' },
    }).catch(console.error);
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'completed':
        return '#10B981';
      case 'failed':
        return '#EF4444';
      case 'syncing':
        return '#3B82F6';
      default:
        return '#F59E0B';
    }
  };

  return (
    <ScrollView
      contentContainerStyle={styles.container}
      accessibilityLabel="Background Sync screen">
      <View style={styles.header}>
        <Text style={styles.title}>Background Sync</Text>
        <Text style={styles.subtitle}>Sync data while the app is closed.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {supported ? 'Background Sync is supported.' : 'Background Sync unavailable.'}
        </Text>

        {loading && (
          <View style={styles.loadingRow}>
            <ActivityIndicator size="small" color="#1F7A8C" />
            <Text style={styles.loadingText}>Syncing…</Text>
          </View>
        )}

        <Button
          title="Queue Task"
          onPress={handleQueueTask}
          disabled={!supported || loading}
          accessibilityLabel="Queue sync task"
        />

        <Button
          title="Sync Now"
          onPress={syncNow}
          disabled={!supported || loading || tasks.length === 0}
          accessibilityLabel="Start synchronization"
        />

        <FlatList
          data={tasks}
          keyExtractor={item => item.id}
          scrollEnabled={false}
          renderItem={({ item }) => (
            <View style={styles.taskItem}>
              <View style={styles.taskHeader}>
                <Text style={styles.taskType}>{item.type}</Text>
                <View
                  style={[styles.statusBadge, { backgroundColor: getStatusColor(item.status) }]}>
                  <Text style={styles.statusText}>{item.status}</Text>
                </View>
              </View>
              <Text style={styles.taskId}>ID: {item.id}</Text>
            </View>
          )}
        />

        <Text style={styles.note}>
          Background Sync keeps your data current even when the app is closed.
        </Text>
      </View>
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: { padding: 20, backgroundColor: '#ffffff' },
  header: { marginBottom: 20 },
  title: { fontSize: 28, fontWeight: '700', color: '#111827' },
  subtitle: { marginTop: 8, fontSize: 15, color: '#4b5563' },
  body: { gap: 14 },
  status: { fontSize: 14, color: '#374151' },
  loadingRow: { flexDirection: 'row', alignItems: 'center', gap: 10 },
  loadingText: { fontSize: 13, color: '#6B7280' },
  taskItem: { padding: 12, backgroundColor: '#F3F4F6', borderRadius: 8, marginBottom: 10 },
  taskHeader: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center' },
  taskType: { fontSize: 14, fontWeight: '600', color: '#111827' },
  statusBadge: { paddingHorizontal: 10, paddingVertical: 4, borderRadius: 4 },
  statusText: { fontSize: 11, color: '#ffffff', fontWeight: '500' },
  taskId: { fontSize: 11, color: '#6B7280', marginTop: 4 },
  note: { fontSize: 12, color: '#6B7280', fontStyle: 'italic' },
});
