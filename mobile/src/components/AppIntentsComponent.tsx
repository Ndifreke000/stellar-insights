import React from 'react';
import {
  ActivityIndicator,
  Button,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useAppIntents } from '@features/app_intents/useAppIntents';

const SAMPLE_INTENTS: { name: string; params: Record<string, string> }[] = [
  { name: 'ViewDashboard', params: { screen: 'dashboard' } },
  { name: 'CheckAnchors', params: { filter: 'active' } },
  { name: 'ShowCorridors', params: { region: 'all' } },
];

export const AppIntentsComponent: React.FC = () => {
  const { isSupported, loading, error, intents, executeIntent, clearIntents } = useAppIntents();

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="App intents screen">
      <View style={styles.header}>
        <Text style={styles.title}>App Intents</Text>
        <Text style={styles.subtitle}>Trigger system-level intents for Stellar features.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {isSupported ? 'App Intents are available on this device.' : 'App Intents are not supported on this platform.'}
        </Text>

        {error ? (
          <Text style={styles.errorText} accessibilityRole="alert">{error}</Text>
        ) : null}

        {loading ? (
          <View style={styles.loaderRow}>
            <ActivityIndicator size="small" color="#059669" />
            <Text style={styles.loadingText}>Executing intent…</Text>
          </View>
        ) : null}

        <View style={styles.intentList} accessible accessibilityLabel="Available intents">
          <Text style={styles.listTitle}>Available Intents</Text>
          {SAMPLE_INTENTS.map(intent => (
            <View key={intent.name} style={styles.intentRow}>
              <Text style={styles.intentName}>{intent.name}</Text>
              <Button
                title="Run"
                onPress={() => void executeIntent(intent.name, intent.params)}
                disabled={!isSupported || loading}
                accessibilityLabel={`Execute ${intent.name} intent`}
              />
            </View>
          ))}
        </View>

        <View style={styles.logCard} accessible accessibilityRole="summary" accessibilityLabel="Intent execution log">
          <View style={styles.logHeader}>
            <Text style={styles.listTitle}>Execution Log</Text>
            {intents.length > 0 ? (
              <Button title="Clear" onPress={() => void clearIntents()} accessibilityLabel="Clear intent log" />
            ) : null}
          </View>
          {intents.length === 0 ? (
            <Text style={styles.emptyText}>No intents executed yet.</Text>
          ) : (
            intents.map(item => (
              <View key={item.id} style={styles.logRow} accessible accessibilityRole="text"
                accessibilityLabel={`${item.name} ${item.status} at ${item.executedAt}`}>
                <Text style={[styles.logName, item.status === 'failed' && styles.failed]}>{item.name}</Text>
                <Text style={styles.logMeta}>{item.status} · {new Date(item.executedAt).toLocaleTimeString()}</Text>
              </View>
            ))
          )}
        </View>
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
  status: { fontSize: 14, color: '#059669' },
  errorText: { fontSize: 13, color: '#b91c1c', marginTop: 8 },
  loaderRow: { flexDirection: 'row', alignItems: 'center', gap: 10, marginVertical: 12 },
  loadingText: { color: '#059669' },
  intentList: { padding: 16, borderRadius: 12, backgroundColor: '#f0fdf4', borderWidth: 1, borderColor: '#bbf7d0' },
  intentRow: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#d1fae5' },
  intentName: { fontSize: 14, fontWeight: '600', color: '#111827' },
  logCard: { marginTop: 8, padding: 16, borderRadius: 12, backgroundColor: '#f8fafc', borderWidth: 1, borderColor: '#e2e8f0' },
  logHeader: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 },
  listTitle: { fontSize: 15, fontWeight: '600', color: '#0f172a' },
  emptyText: { fontSize: 14, color: '#475569' },
  logRow: { paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#e2e8f0' },
  logName: { fontSize: 14, fontWeight: '600', color: '#111827' },
  failed: { color: '#b91c1c' },
  logMeta: { fontSize: 12, color: '#64748b', marginTop: 2 },
});
