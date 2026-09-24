import React from 'react';
import { Platform, StyleSheet, Text, View } from 'react-native';
import { useAppStore } from '../store/appStore';

export const OfflineBanner: React.FC = () => {
  const { isOnline } = useAppStore();

  if (isOnline) {
    return null;
  }

  return (
    <View
      style={styles.container}
      accessibilityRole="alert"
      accessibilityLabel="No internet connection — offline mode active"
      accessibilityLiveRegion="polite"
    >
      <Text style={styles.icon} accessibilityElementsHidden>
        ⚡
      </Text>
      <View style={styles.textContainer}>
        <Text style={styles.title}>No Internet Connection</Text>
        <Text style={styles.message}>
          You&apos;re offline. Cached data is shown where available.
        </Text>
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    backgroundColor: '#1f2937',
    paddingHorizontal: 16,
    paddingVertical: Platform.select({ ios: 14, android: 12, default: 12 }),
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  icon: {
    fontSize: 20,
  },
  textContainer: {
    flex: 1,
  },
  title: {
    fontSize: 14,
    fontWeight: '700',
    color: '#f9fafb',
    marginBottom: 2,
  },
  message: {
    fontSize: 12,
    color: '#d1d5db',
    lineHeight: 16,
  },
});
