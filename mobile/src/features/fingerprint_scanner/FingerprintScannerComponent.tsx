import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ActivityIndicator, TouchableOpacity, AccessibilityInfo, AccessibilityRole } from 'react-native';
import { useFingerprintScanner } from './useFingerprintScanner';

export const FingerprintScannerComponent = () => {
  const {
    isScanning,
    isAuthenticated,
    error,
    startScan,
    stopScan,
    resetResult,
    hasPermission,
    requestPermission,
    isBiometricAvailable,
  } = useFingerprintScanner();

  const [isInitialized, setIsInitialized] = useState(false);
  const [bioAvailable, setBioAvailable] = useState(false);

  useEffect(() => {
    const initialize = async () => {
      try {
        const bioAvail = await isBiometricAvailable();
        setBioAvailable(bioAvail);

        if (bioAvail) {
          const hasPermissionResult = await hasPermission();
          if (!hasPermissionResult) {
            await requestPermission();
          }
        }
      } catch (err) {
        console.warn('Failed to initialize fingerprint scanner:', err);
      }
      setIsInitialized(true);
    };

    initialize();
  }, []);

  const handleStartScan = async () => {
    try {
      await startScan();
    } catch (err) {
      console.error('Failed to start fingerprint scan:', err);
    }
  };

  if (!isInitialized) {
    return (
      <View style={styles.container} accessibilityRole="status">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing fingerprint scanner">
          Initializing Fingerprint Scanner...
        </Text>
      </View>
    );
  }

  if (!bioAvailable) {
    return (
      <View style={styles.container} accessibilityRole="alert">
        <View style={styles.errorContainer}>
          <Text style={styles.errorText}>Biometric authentication not available on this device</Text>
        </View>
      </View>
    );
  }

  return (
    <View style={styles.container} accessibilityRole="main">
      <View
        style={styles.header}
        accessible={true}
        accessibilityLabel="Fingerprint Scanner Interface"
        accessibilityRole="header"
      >
        <Text style={styles.title}>Fingerprint Scanner</Text>
      </View>

      <View style={styles.content}>
        {error && (
          <View
            style={styles.errorContainer}
            accessible={true}
            accessibilityRole="alert"
            accessibilityLabel={`Error: ${error}`}
          >
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        {isAuthenticated && (
          <View
            style={styles.successContainer}
            accessible={true}
            accessibilityRole="status"
            accessibilityLabel="Authentication successful"
          >
            <Text style={styles.successText}>✓ Authentication Successful</Text>
            <TouchableOpacity
              style={styles.resetButton}
              onPress={resetResult}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Reset authentication"
            >
              <Text style={styles.resetButtonText}>Reset</Text>
            </TouchableOpacity>
          </View>
        )}

        {!isScanning && !isAuthenticated && (
          <TouchableOpacity
            style={styles.scanButton}
            onPress={handleStartScan}
            disabled={isScanning}
            accessible={true}
            accessibilityRole="button"
            accessibilityLabel="Start fingerprint scan"
            accessibilityHint="Tap to authenticate with your fingerprint"
          >
            {isScanning ? (
              <ActivityIndicator size="small" color="#fff" />
            ) : (
              <Text style={styles.scanButtonText}>Scan Fingerprint</Text>
            )}
          </TouchableOpacity>
        )}

        {isScanning && (
          <View style={styles.scanningContainer}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.scanningText} accessibilityLabel="Place your finger on the sensor">
              Place your finger on the sensor
            </Text>
            <TouchableOpacity
              style={styles.stopButton}
              onPress={stopScan}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Cancel scan"
            >
              <Text style={styles.stopButtonText}>Cancel</Text>
            </TouchableOpacity>
          </View>
        )}
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
    justifyContent: 'center',
    alignItems: 'center',
    padding: 16,
  },
  header: {
    paddingVertical: 16,
    paddingHorizontal: 16,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  title: {
    fontSize: 24,
    fontWeight: '600',
    color: '#333',
  },
  content: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingHorizontal: 16,
  },
  loadingText: {
    marginTop: 12,
    fontSize: 16,
    color: '#666',
  },
  errorContainer: {
    backgroundColor: '#ffebee',
    borderRadius: 8,
    padding: 16,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#d32f2f',
    minWidth: '80%',
  },
  errorText: {
    color: '#d32f2f',
    fontSize: 14,
    fontWeight: '500',
  },
  successContainer: {
    backgroundColor: '#e8f5e9',
    borderRadius: 8,
    padding: 16,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#388e3c',
    minWidth: '80%',
    alignItems: 'center',
  },
  successText: {
    color: '#2e7d32',
    fontSize: 18,
    fontWeight: '700',
    marginBottom: 12,
  },
  scanButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 14,
    paddingHorizontal: 32,
    minWidth: '70%',
  },
  scanButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
    textAlign: 'center',
  },
  scanningContainer: {
    alignItems: 'center',
  },
  scanningText: {
    marginTop: 16,
    fontSize: 16,
    color: '#666',
    marginBottom: 20,
  },
  stopButton: {
    backgroundColor: '#d32f2f',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 32,
  },
  stopButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  resetButton: {
    backgroundColor: '#f57c00',
    borderRadius: 6,
    paddingVertical: 10,
    paddingHorizontal: 16,
  },
  resetButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '600',
    textAlign: 'center',
  },
});
