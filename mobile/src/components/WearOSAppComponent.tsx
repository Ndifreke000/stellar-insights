import React, { useState } from 'react';
import { ActivityIndicator, Button, ScrollView, StyleSheet, Text, TextInput, View } from 'react-native';
import { useWearOSApp } from '@hooks/useWearOSApp';

const SAMPLE_MESSAGES: { type: string; payload: Record<string, string> }[] = [
  { type: 'sync', payload: { screen: 'dashboard' } },
  { type: 'notification', payload: { message: 'Corridor update' } },
];

export const WearOSAppComponent: React.FC = () => {
  const { isSupported, loading, error, messages, sendMessage, clearMessages } = useWearOSApp();
  const [customType, setCustomType] = useState('');

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="Wear OS App screen">
      <Text style={styles.title}>Wear OS App</Text>
      <Text style={styles.subtitle}>Android Wear companion for PayRaider.</Text>
      <Text style={styles.status} accessibilityRole="text">
        {isSupported ? 'Wear OS supported on this device.' : 'Wear OS App requires Android.'}
      </Text>

      {error ? <Text style={styles.error} accessibilityRole="alert">{error}</Text> : null}
      {loading ? (
        <View style={styles.row}><ActivityIndicator size="small" color="#059669" /><Text style={styles.loadingText}>Sending…</Text></View>
      ) : null}

      <Text style={styles.sectionTitle}>Sample Messages</Text>
      {SAMPLE_MESSAGES.map(msg => (
        <View key={msg.type} style={styles.itemRow}>
          <Text style={styles.itemName}>{msg.type}</Text>
          <Button title="Send" onPress={() => void sendMessage(msg.type, msg.payload)} disabled={!isSupported || loading} accessibilityLabel={`Send ${msg.type} to Wear OS`} />
        </View>
      ))}

      <Text style={styles.sectionTitle}>Custom Message</Text>
      <TextInput style={styles.input} placeholder="Message type" value={customType} onChangeText={setCustomType} accessibilityLabel="Custom message type input" />
      <Button title="Send Custom" onPress={() => { void sendMessage(customType, {}); setCustomType(''); }} disabled={!isSupported || loading || !customType} accessibilityLabel="Send custom Wear OS message" />

      <View style={styles.logCard} accessibilityRole="summary" accessibilityLabel="Wear OS message log">
        <View style={styles.row}>
          <Text style={styles.sectionTitle}>Message Log</Text>
          {messages.length > 0 ? <Button title="Clear" onPress={() => void clearMessages()} accessibilityLabel="Clear message log" /> : null}
        </View>
        {messages.length === 0
          ? <Text style={styles.empty}>No messages sent yet.</Text>
          : messages.map(m => (
            <View key={m.id} style={styles.logRow} accessibilityRole="text" accessibilityLabel={`${m.type} ${m.status}`}>
              <Text style={[styles.itemName, m.status === 'failed' && styles.failed]}>{m.type}</Text>
              <Text style={styles.meta}>{m.status} · {new Date(m.sentAt).toLocaleTimeString()}</Text>
            </View>
          ))}
      </View>
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: { padding: 20, backgroundColor: '#fff' },
  title: { fontSize: 28, fontWeight: '700', color: '#111827', marginBottom: 4 },
  subtitle: { fontSize: 15, color: '#4b5563', marginBottom: 12 },
  status: { fontSize: 14, color: '#059669', marginBottom: 8 },
  error: { fontSize: 13, color: '#b91c1c', marginBottom: 8 },
  row: { flexDirection: 'row', alignItems: 'center', gap: 10, marginBottom: 8 },
  loadingText: { color: '#059669' },
  sectionTitle: { fontSize: 15, fontWeight: '600', color: '#0f172a', marginTop: 16, marginBottom: 8 },
  itemRow: { flexDirection: 'row', justifyContent: 'space-between', alignItems: 'center', paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#e2e8f0' },
  itemName: { fontSize: 14, fontWeight: '600', color: '#111827' },
  input: { borderWidth: 1, borderColor: '#d1d5db', borderRadius: 8, padding: 10, marginBottom: 8, fontSize: 14 },
  logCard: { marginTop: 16, padding: 16, borderRadius: 12, backgroundColor: '#f8fafc', borderWidth: 1, borderColor: '#e2e8f0' },
  logRow: { paddingVertical: 8, borderBottomWidth: 1, borderBottomColor: '#e2e8f0' },
  failed: { color: '#b91c1c' },
  meta: { fontSize: 12, color: '#64748b', marginTop: 2 },
  empty: { fontSize: 14, color: '#475569' },
});
