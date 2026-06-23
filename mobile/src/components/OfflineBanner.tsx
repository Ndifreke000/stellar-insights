import React from 'react';
import { Platform, StyleSheet, Text, View } from 'react-native';
import { useAppStore } from '@store/appStore';

export const OfflineBanner: React.FC = () => {
  const isOnline = useAppStore(state => state.isOnline);

  if (isOnline) {
    return null;
  }

  return (
    <View style={styles.banner} accessibilityRole="alert" accessibilityLabel="No internet connection">
      <Text style={styles.icon}>⚠</Text>
      <View style={styles.textContainer}>
        <Text style={styles.title}>No Internet Connection</Text>
        <Text style={styles.subtitle}>Some features may be unavailable. Your changes will sync when you're back online.</Text>
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  banner: {
    backgroundColor: '#1f2937',
    paddingHorizontal: 16,
    paddingVertical: Platform.select({ ios: 14, android: 12, default: 12 }),
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  icon: {
    fontSize: 20,
    color: '#f59e0b',
  },
  textContainer: {
    flex: 1,
  },
  title: {
    color: '#f9fafb',
    fontWeight: '700',
    fontSize: 14,
  },
  subtitle: {
    color: '#9ca3af',
    fontSize: 12,
    marginTop: 2,
  },
});
