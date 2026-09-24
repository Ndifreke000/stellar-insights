import React from 'react';
import { ActivityIndicator, Platform, StyleSheet, Text, View } from 'react-native';
import { useSplashScreen } from '@hooks/useSplashScreen';

export const SplashScreen: React.FC = () => {
  const { status, error } = useSplashScreen();

  return (
    <View
      style={styles.container}
      accessibilityRole="none"
      accessibilityLabel={status === 'error' ? `Initialization error: ${error}` : 'Loading PayRaider'}
    >
      <Text style={styles.title} accessibilityRole="header">
        PayRaider
      </Text>

      {status === 'loading' && (
        <ActivityIndicator
          size="large"
          color="#ffffff"
          style={styles.indicator}
          accessibilityLabel="Loading"
        />
      )}

      {status === 'error' && (
        <Text style={styles.error} accessibilityRole="alert">
          {error ?? 'Something went wrong. Please restart the app.'}
        </Text>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#1a1a2e',
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 24,
  },
  title: {
    fontSize: 32,
    fontWeight: '700',
    color: '#ffffff',
    letterSpacing: 0.5,
    marginBottom: Platform.select({ ios: 32, android: 28, default: 28 }),
  },
  indicator: {
    marginTop: 8,
  },
  error: {
    marginTop: 16,
    color: '#fca5a5',
    fontSize: 14,
    textAlign: 'center',
    lineHeight: 20,
  },
});
