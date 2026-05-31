import React from 'react';
import { ActivityIndicator, Alert, Button, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useHandoffSupport } from '@features/handoff_support/useHandoffSupport';

export const HandoffSupportComponent: React.FC = () => {
  const { loading, error, supported, activity, continueActivity } = useHandoffSupport();

  React.useEffect(() => {
    if (error) {
      Alert.alert('Handoff Error', error, [{ text: 'OK' }], { cancelable: true });
    }
  }, [error]);

  const handleContinue = () => {
    continueActivity({
      type: 'com.stellar-insights.dashboard',
      userInfo: { screen: 'dashboard', timestamp: Date.now() },
      needsSave: true,
    }).catch(console.error);
  };

  return (
    <ScrollView
      contentContainerStyle={styles.container}
      accessibilityLabel="Handoff Support screen">
      <View style={styles.header}>
        <Text style={styles.title}>Handoff Support</Text>
        <Text style={styles.subtitle}>Continue activities across your Apple devices.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {supported ? 'Handoff is available.' : 'Handoff is not available.'}
        </Text>

        {loading && (
          <View style={styles.loadingRow}>
            <ActivityIndicator size="small" color="#1F7A8C" />
            <Text style={styles.loadingText}>Initializing Handoff…</Text>
          </View>
        )}

        <Button
          title="Continue Activity"
          onPress={handleContinue}
          disabled={!supported || loading}
          accessibilityLabel="Start Handoff activity"
        />

        {activity && (
          <View style={styles.activityDisplay}>
            <Text style={styles.activityLabel}>Type: {activity.type}</Text>
            <Text style={styles.activityLabel}>
              Save needed: {activity.needsSave ? 'Yes' : 'No'}
            </Text>
          </View>
        )}

        <Text style={styles.note}>
          Handoff allows you to start a task on one Apple device and continue it on another.
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
  activityDisplay: { padding: 15, backgroundColor: '#F3F4F6', borderRadius: 8 },
  activityLabel: { fontSize: 12, color: '#374151', marginBottom: 4 },
  note: { fontSize: 12, color: '#6B7280', fontStyle: 'italic' },
});
