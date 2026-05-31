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
import { useGeofencing } from '@features/geofencing/useGeofencing';

export const GeofencingComponent: React.FC = () => {
  const { loading, error, supported, fences, addFence, removeFence } = useGeofencing();

  React.useEffect(() => {
    if (error) {
      Alert.alert('Geofencing Error', error, [{ text: 'OK' }], { cancelable: true });
    }
  }, [error]);

  const handleAddFence = () => {
    addFence({
      id: `fence_${Date.now()}`,
      latitude: 37.7749,
      longitude: -122.4194,
      radius: 100,
      name: 'San Francisco',
    }).catch(console.error);
  };

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="Geofencing screen">
      <View style={styles.header}>
        <Text style={styles.title}>Geofencing</Text>
        <Text style={styles.subtitle}>Set location-based alerts and triggers.</Text>
      </View>

      <View style={styles.body}>
        <Text style={styles.status} accessibilityRole="text">
          {supported ? 'Geofencing is supported on this device.' : 'Geofencing unavailable.'}
        </Text>

        {loading && (
          <View style={styles.loadingRow}>
            <ActivityIndicator size="small" color="#1F7A8C" />
            <Text style={styles.loadingText}>Processing…</Text>
          </View>
        )}

        <Button
          title="Add Geofence"
          onPress={handleAddFence}
          disabled={!supported || loading}
          accessibilityLabel="Add new geofence"
        />

        <FlatList
          data={fences}
          keyExtractor={item => item.id}
          scrollEnabled={false}
          renderItem={({ item }) => (
            <View style={styles.fenceItem}>
              <Text style={styles.fenceName}>{item.name}</Text>
              <Text style={styles.fenceDetail}>Radius: {item.radius}m</Text>
              <Button title="Remove" onPress={() => removeFence(item.id)} color="#EF4444" />
            </View>
          )}
        />

        <Text style={styles.note}>
          Geofencing enables location-based notifications and actions while respecting privacy.
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
  fenceItem: { padding: 12, backgroundColor: '#F3F4F6', borderRadius: 8, marginBottom: 10 },
  fenceName: { fontSize: 14, fontWeight: '600', color: '#111827', marginBottom: 4 },
  fenceDetail: { fontSize: 12, color: '#6B7280', marginBottom: 8 },
  note: { fontSize: 12, color: '#6B7280', fontStyle: 'italic' },
});
