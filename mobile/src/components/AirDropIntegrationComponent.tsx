import React from 'react';
import {
  ActivityIndicator,
  Button,
  ScrollView,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { useAirDropIntegration } from '@features/airdrop_integration/useAirDropIntegration';

export const AirDropIntegrationComponent: React.FC = () => {
  const { isSupported, loading, error, history, shareContent, clearHistory } =
    useAirDropIntegration();

  const handleShare = () => {
    void shareContent(
      'PayRaider',
      'Check out real-time Stellar payment analytics!',
      'https://stellar.org',
    );
  };

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="AirDrop integration screen">
      <View style={styles.header}>
        <Text style={styles.title}>AirDrop Integration</Text>
        <Text style={styles.subtitle}>Share Stellar data with nearby devices via AirDrop or system share sheet.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {isSupported
            ? 'Sharing is available on this device.'
            : 'Sharing is not supported on this platform.'}
        </Text>

        {error ? (
          <Text style={styles.errorText} accessibilityRole="alert">
            {error}
          </Text>
        ) : null}

        {loading ? (
          <View style={styles.loaderRow}>
            <ActivityIndicator size="small" color="#0369a1" />
            <Text style={styles.loadingText}>Processing share…</Text>
          </View>
        ) : null}

        <View style={styles.buttonGroup}>
          <Button
            title="Share via AirDrop"
            onPress={handleShare}
            disabled={!isSupported || loading}
            accessibilityLabel="Share content via AirDrop"
          />
        </View>

        <View style={styles.historyCard} accessible accessibilityRole="summary" accessibilityLabel="Share history">
          <View style={styles.historyHeader}>
            <Text style={styles.listTitle}>Share History</Text>
            {history.length > 0 ? (
              <Button title="Clear" onPress={() => void clearHistory()} accessibilityLabel="Clear share history" />
            ) : null}
          </View>
          {history.length === 0 ? (
            <Text style={styles.emptyText}>No shares yet.</Text>
          ) : (
            history.map(item => (
              <View key={item.id} style={styles.historyRow} accessible accessibilityRole="text" accessibilityLabel={`Shared ${item.title} at ${item.sharedAt}`}>
                <Text style={styles.historyTitle}>{item.title}</Text>
                <Text style={styles.historyMeta}>{new Date(item.sharedAt).toLocaleString()}</Text>
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
  status: { fontSize: 14, color: '#0369a1' },
  errorText: { fontSize: 13, color: '#b91c1c', marginTop: 8 },
  loaderRow: { flexDirection: 'row', alignItems: 'center', gap: 10, marginVertical: 12 },
  loadingText: { color: '#0369a1' },
  buttonGroup: { marginTop: 10 },
  historyCard: { marginTop: 20, padding: 16, borderRadius: 12, backgroundColor: '#f8fafc', borderWidth: 1, borderColor: '#e2e8f0' },
  historyHeader: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 },
  listTitle: { fontSize: 15, fontWeight: '600', color: '#0f172a' },
  emptyText: { fontSize: 14, color: '#475569' },
  historyRow: { paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#e2e8f0' },
  historyTitle: { fontSize: 14, fontWeight: '600', color: '#111827' },
  historyMeta: { fontSize: 12, color: '#64748b', marginTop: 2 },
});
