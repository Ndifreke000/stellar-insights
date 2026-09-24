import React from 'react';
import { Platform, Pressable, ScrollView, StyleSheet, Text, View } from 'react-native';
import { useSettingsScreen } from '@hooks/useSettingsScreen';

export const SettingsScreen: React.FC = () => {
  const {
    theme,
    network,
    isOnline,
    isSyncing,
    pendingMessage,
    error,
    platformLabel,
    toggleTheme,
    toggleNetwork,
    clearMessage,
  } = useSettingsScreen();

  return (
    <ScrollView style={styles.container} contentContainerStyle={styles.content} accessibilityLabel="Settings screen">
      <View style={styles.header}>
        <Text style={styles.title}>Settings Screen</Text>
        <Text style={styles.subtitle}>{platformLabel} preferences and account controls</Text>
      </View>

      {!isOnline ? (
        <View style={styles.offlineBanner} accessibilityRole="alert" accessibilityLabel="Settings offline mode active">
          <Text style={styles.offlineText}>Offline mode active. Changes are saved locally where possible.</Text>
        </View>
      ) : null}

      {pendingMessage ? (
        <Pressable style={styles.messageCard} onPress={clearMessage} accessibilityRole="button" accessibilityLabel={`Dismiss message ${pendingMessage}`}>
          <Text style={styles.messageText}>{pendingMessage}</Text>
        </Pressable>
      ) : null}

      {error ? (
        <View style={styles.errorCard} accessibilityRole="alert" accessibilityLabel={`Settings error ${error}`}>
          <Text style={styles.errorText}>{error}</Text>
        </View>
      ) : null}

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Appearance</Text>
        <Pressable style={styles.option} onPress={toggleTheme} accessibilityRole="button" accessibilityLabel={`Toggle theme. Current theme is ${theme}`}>
          <View>
            <Text style={styles.optionLabel}>Theme</Text>
            <Text style={styles.optionDescription}>Use a platform-friendly {theme} interface</Text>
          </View>
          <Text style={styles.optionValue}>{theme}</Text>
        </Pressable>
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Network</Text>
        <Pressable style={styles.option} onPress={toggleNetwork} accessibilityRole="button" accessibilityLabel={`Toggle Stellar network. Current network is ${network}`}>
          <View>
            <Text style={styles.optionLabel}>Current Network</Text>
            <Text style={styles.optionDescription}>Switch between testnet and mainnet</Text>
          </View>
          <Text style={styles.optionValue}>{network}</Text>
        </Pressable>
        <View style={styles.option} accessible accessibilityLabel={`Sync status ${isSyncing ? 'syncing' : 'idle'}`}>
          <View>
            <Text style={styles.optionLabel}>Sync Status</Text>
            <Text style={styles.optionDescription}>Offline queue and cached data</Text>
          </View>
          <Text style={styles.optionValue}>{isSyncing ? 'syncing' : 'idle'}</Text>
        </View>
      </View>

      <View style={styles.section}>
        <Text style={styles.sectionTitle}>Support</Text>
        <View style={styles.option} accessible accessibilityLabel={`Device platform ${platformLabel}`}>
          <View>
            <Text style={styles.optionLabel}>Platform</Text>
            <Text style={styles.optionDescription}>Optimized for iOS 14+ and Android 8+</Text>
          </View>
          <Text style={styles.optionValue}>{platformLabel}</Text>
        </View>
      </View>
    </ScrollView>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  content: {
    paddingBottom: Platform.select({ ios: 32, android: 24, default: 24 }),
  },
  header: {
    padding: 20,
    backgroundColor: '#fff',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  subtitle: {
    color: '#666',
    marginTop: 6,
  },
  offlineBanner: {
    backgroundColor: '#fef3c7',
    margin: 16,
    padding: 14,
    borderRadius: 12,
  },
  offlineText: {
    color: '#92400e',
  },
  messageCard: {
    backgroundColor: '#dcfce7',
    marginHorizontal: 16,
    marginTop: 16,
    padding: 14,
    borderRadius: 12,
  },
  messageText: {
    color: '#166534',
  },
  errorCard: {
    backgroundColor: '#fee2e2',
    marginHorizontal: 16,
    marginTop: 16,
    padding: 14,
    borderRadius: 12,
  },
  errorText: {
    color: '#991b1b',
  },
  section: {
    marginTop: 20,
    backgroundColor: '#fff',
    borderRadius: Platform.OS === 'ios' ? 14 : 8,
    marginHorizontal: 16,
    overflow: 'hidden',
    elevation: Platform.OS === 'android' ? 2 : 0,
  },
  sectionTitle: {
    fontSize: 13,
    fontWeight: '600',
    color: '#666',
    paddingHorizontal: 16,
    paddingVertical: 10,
    textTransform: 'uppercase',
  },
  option: {
    minHeight: 56,
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    paddingHorizontal: 16,
    paddingVertical: 12,
    borderTopWidth: 1,
    borderTopColor: '#f0f0f0',
    gap: 12,
  },
  optionLabel: {
    fontSize: 16,
    fontWeight: '600',
  },
  optionDescription: {
    color: '#666',
    marginTop: 4,
  },
  optionValue: {
    fontSize: 16,
    color: '#007AFF',
    textTransform: 'capitalize',
  },
});
