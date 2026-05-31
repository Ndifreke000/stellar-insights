import React from 'react';
import {
  ActivityIndicator,
  Alert,
  ScrollView,
  StyleSheet,
  Text,
  TouchableOpacity,
  View,
} from 'react-native';
import { useForceTouch } from '@features/force_touch/useForceTouch';

export const ForceTouchComponent: React.FC = () => {
  const { loading, error, supported, touchData, handlePress } = useForceTouch();

  React.useEffect(() => {
    if (error) {
      Alert.alert('Force Touch Error', error, [{ text: 'OK' }], { cancelable: true });
    }
  }, [error]);

  const pressureLevel = touchData?.force ? Math.round(touchData.force * 100) : 0;

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="Force Touch screen">
      <View style={styles.header}>
        <Text style={styles.title}>Force Touch</Text>
        <Text style={styles.subtitle}>Press with varying pressure to trigger actions.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {supported ? 'Force Touch is supported on this device.' : 'Force Touch unavailable.'}
        </Text>

        {loading && (
          <View style={styles.loadingRow}>
            <ActivityIndicator size="small" color="#1F7A8C" />
            <Text style={styles.loadingText}>Detecting pressure…</Text>
          </View>
        )}

        <TouchableOpacity
          style={[styles.touchArea, !supported && styles.disabled]}
          onPress={handlePress}
          disabled={!supported || loading}
          accessibilityLabel="Force touch detection area"
          activeOpacity={0.7}>
          <Text style={styles.touchText}>Press Here</Text>
        </TouchableOpacity>

        {touchData && (
          <View style={styles.dataDisplay}>
            <Text style={styles.dataLabel}>Pressure: {pressureLevel}%</Text>
            <Text style={styles.dataLabel}>X: {touchData.x.toFixed(0)}</Text>
            <Text style={styles.dataLabel}>Y: {touchData.y.toFixed(0)}</Text>
          </View>
        )}

        <Text style={styles.note}>
          Force Touch enables pressure-sensitive interactions for enhanced user engagement.
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
  touchArea: {
    padding: 40,
    backgroundColor: '#E5E7EB',
    borderRadius: 12,
    justifyContent: 'center',
    alignItems: 'center',
  },
  disabled: { opacity: 0.5 },
  touchText: { fontSize: 18, fontWeight: '600', color: '#1F7A8C' },
  dataDisplay: { padding: 15, backgroundColor: '#F3F4F6', borderRadius: 8 },
  dataLabel: { fontSize: 12, color: '#374151', marginBottom: 4 },
  note: { fontSize: 12, color: '#6B7280', fontStyle: 'italic' },
});
