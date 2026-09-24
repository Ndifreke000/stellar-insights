import React from 'react';
import { Platform, Pressable, StyleSheet, Text, View } from 'react-native';
import { useNetworkStatusIndicator } from '@hooks/useNetworkStatusIndicator';

export const NetworkStatusIndicator: React.FC = () => {
  const { status, message, isVisible, platformOffset, dismiss } = useNetworkStatusIndicator();

  if (!isVisible) {
    return null;
  }

  return (
    <View
      style={[styles.container, styles[status], { marginTop: platformOffset }]}
      accessibilityRole="alert"
      accessibilityLabel={`Network Status Indicator ${message}`}
    >
      <Text style={styles.title}>Network Status Indicator</Text>
      <Text style={styles.message}>{message}</Text>
      <Pressable onPress={dismiss} accessibilityRole="button" accessibilityLabel="Dismiss network status indicator" style={styles.dismissButton}>
        <Text style={styles.dismissText}>Dismiss</Text>
      </Pressable>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    marginHorizontal: 12,
    marginBottom: 8,
    paddingHorizontal: 14,
    paddingVertical: Platform.select({ ios: 12, android: 10, default: 10 }),
    borderRadius: Platform.OS === 'ios' ? 14 : 8,
    flexDirection: 'row',
    alignItems: 'center',
    gap: 10,
    elevation: Platform.OS === 'android' ? 3 : 0,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: Platform.OS === 'ios' ? 0.12 : 0,
    shadowRadius: 4,
  },
  online: {
    backgroundColor: '#dcfce7',
  },
  offline: {
    backgroundColor: '#fef3c7',
  },
  syncing: {
    backgroundColor: '#dbeafe',
  },
  title: {
    fontWeight: '700',
    color: '#111827',
  },
  message: {
    color: '#374151',
    flex: 1,
  },
  dismissButton: {
    minHeight: 44,
    justifyContent: 'center',
  },
  dismissText: {
    color: '#2563eb',
    fontWeight: '600',
  },
});
