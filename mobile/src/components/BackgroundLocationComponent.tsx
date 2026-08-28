import React from 'react';
import {
  View, Text, TouchableOpacity, ActivityIndicator,
  StyleSheet, ScrollView,
} from 'react-native';
import { useBackgroundLocation } from '../hooks/useBackgroundLocation';
import type { BackgroundLocationConfig } from '../features/background_location/types';

interface BackgroundLocationComponentProps {
  config?: Partial<BackgroundLocationConfig>;
}

export const BackgroundLocationComponent: React.FC<BackgroundLocationComponentProps> = ({ config }) => {
  const {
    permissionStatus, isTracking, lastLocation, locationHistory,
    error, isLoading, requestPermission, startTracking, stopTracking, clearHistory,
  } = useBackgroundLocation(config);

  const permissionLabels: Record<string, string> = {
    not_requested: 'Not requested',
    foreground_only: 'Foreground only (background denied)',
    background_granted: 'Background access granted',
    denied: 'Permission denied',
    unavailable: 'Location unavailable on this device',
  };

  return (
    <ScrollView contentContainerStyle={styles.container} accessibilityLabel="Background location settings">
      <View style={styles.section} accessibilityLabel="Permission status">
        <Text style={styles.label} accessibilityRole="header">Location Permission</Text>
        <Text
          style={[styles.statusText, permissionStatus === 'background_granted' && styles.statusGreen, permissionStatus === 'denied' && styles.statusRed]}
          accessibilityLiveRegion="polite"
        >
          {permissionLabels[permissionStatus]}
        </Text>
        {permissionStatus !== 'background_granted' && (
          <TouchableOpacity style={styles.button} onPress={requestPermission} disabled={isLoading} accessibilityRole="button" accessibilityLabel="Request background location permission" accessibilityState={{ disabled: isLoading }}>
            <Text style={styles.buttonText}>Request Permission</Text>
          </TouchableOpacity>
        )}
      </View>

      <View style={styles.section} accessibilityLabel="Tracking controls">
        <Text style={styles.label} accessibilityRole="header">Tracking</Text>
        {isLoading ? (
          <ActivityIndicator size="small" accessibilityLabel="Loading location service" />
        ) : (
          <TouchableOpacity
            style={[styles.button, isTracking && styles.buttonStop]}
            onPress={isTracking ? stopTracking : startTracking}
            disabled={permissionStatus === 'denied' || permissionStatus === 'unavailable'}
            accessibilityRole="button"
            accessibilityLabel={isTracking ? 'Stop location tracking' : 'Start location tracking'}
            accessibilityState={{ disabled: permissionStatus === 'denied' }}
          >
            <Text style={styles.buttonText}>{isTracking ? 'Stop Tracking' : 'Start Tracking'}</Text>
          </TouchableOpacity>
        )}
      </View>

      {error && (
        <View style={styles.errorContainer} accessibilityRole="alert" accessibilityLiveRegion="assertive">
          <Text style={styles.errorText}>{error}</Text>
        </View>
      )}

      {lastLocation && (
        <View style={styles.section} accessibilityLabel="Last known location">
          <Text style={styles.label} accessibilityRole="header">Last Location</Text>
          <Text>Lat: {lastLocation.latitude.toFixed(6)}</Text>
          <Text>Lng: {lastLocation.longitude.toFixed(6)}</Text>
          <Text>Accuracy: ±{lastLocation.accuracy.toFixed(0)}m</Text>
        </View>
      )}

      <View style={styles.section}>
        <Text style={styles.label} accessibilityRole="header">History ({locationHistory.length} points)</Text>
        {locationHistory.length > 0 && (
          <TouchableOpacity style={styles.buttonSecondary} onPress={clearHistory} accessibilityRole="button" accessibilityLabel="Clear location history">
            <Text style={styles.buttonText}>Clear History</Text>
          </TouchableOpacity>
        )}
      </View>
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: { padding: 16, flexGrow: 1 },
  section: { marginBottom: 24 },
  label: { fontSize: 16, fontWeight: '600', marginBottom: 8 },
  statusText: { fontSize: 14, color: '#666', marginBottom: 8 },
  statusGreen: { color: '#22c55e' },
  statusRed: { color: '#ef4444' },
  button: { backgroundColor: '#3b82f6', borderRadius: 8, padding: 12, alignItems: 'center' },
  buttonStop: { backgroundColor: '#ef4444' },
  buttonSecondary: { backgroundColor: '#6b7280', borderRadius: 8, padding: 10, alignItems: 'center' },
  buttonText: { color: '#fff', fontWeight: '600' },
  errorContainer: { backgroundColor: '#fef2f2', borderRadius: 8, padding: 12, marginBottom: 16 },
  errorText: { color: '#dc2626' },
});

export default BackgroundLocationComponent;
