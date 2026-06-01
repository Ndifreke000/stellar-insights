import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ActivityIndicator, TouchableOpacity, AccessibilityInfo, AccessibilityRole } from 'react-native';
import { useBarcodeScanner } from './useBarcodeScanner';

export const BarcodeScannerComponent = () => {
  const {
    isScanning,
    result,
    error,
    startScan,
    stopScan,
    resetResult,
    hasPermission,
    requestPermission,
  } = useBarcodeScanner();

  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    const initialize = async () => {
      const hasPermissionResult = await hasPermission();
      if (!hasPermissionResult) {
        await requestPermission();
      }
      setIsInitialized(true);
    };

    initialize().catch(err => {
      console.warn('Failed to initialize barcode scanner:', err);
      setIsInitialized(true);
    });

    return () => {
      if (isScanning) {
        stopScan();
      }
    };
  }, []);

  const handleStartScan = async () => {
    try {
      await startScan();
    } catch (err) {
      console.error('Failed to start barcode scan:', err);
    }
  };

  if (!isInitialized) {
    return (
      <View style={styles.container} accessibilityRole="status">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing barcode scanner">
          Initializing Barcode Scanner...
        </Text>
      </View>
    );
  }

  return (
    <View style={styles.container} accessibilityRole="main">
      <View
        style={styles.header}
        accessible={true}
        accessibilityLabel="Barcode Scanner Interface"
        accessibilityRole="header"
      >
        <Text style={styles.title}>Barcode Scanner</Text>
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

        {result && (
          <View
            style={styles.resultContainer}
            accessible={true}
            accessibilityRole="status"
            accessibilityLabel={`Barcode scanned: ${result.data}`}
          >
            <Text style={styles.resultLabel}>Scanned Barcode:</Text>
            <Text style={styles.resultValue} selectable={true}>
              {result.data}
            </Text>
            {result.type && (
              <Text style={styles.resultType} accessibilityLabel={`Barcode type: ${result.type}`}>
                Type: {result.type}
              </Text>
            )}
            <TouchableOpacity
              style={styles.clearButton}
              onPress={resetResult}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Clear result"
              accessibilityHint="Clears the current scan result"
            >
              <Text style={styles.clearButtonText}>Clear</Text>
            </TouchableOpacity>
          </View>
        )}

        {!isScanning && !result && (
          <TouchableOpacity
            style={styles.scanButton}
            onPress={handleStartScan}
            disabled={isScanning}
            accessible={true}
            accessibilityRole="button"
            accessibilityLabel="Start barcode scan"
            accessibilityHint="Tap to start scanning barcodes"
          >
            {isScanning ? (
              <ActivityIndicator size="small" color="#fff" />
            ) : (
              <Text style={styles.scanButtonText}>Start Scan</Text>
            )}
          </TouchableOpacity>
        )}

        {isScanning && (
          <View style={styles.scanningContainer}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.scanningText} accessibilityLabel="Scanner is active, please aim at a barcode">
              Scanning... Point at a barcode
            </Text>
            <TouchableOpacity
              style={styles.stopButton}
              onPress={stopScan}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Stop scan"
            >
              <Text style={styles.stopButtonText}>Stop</Text>
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
    padding: 12,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#d32f2f',
  },
  errorText: {
    color: '#d32f2f',
    fontSize: 14,
    fontWeight: '500',
  },
  resultContainer: {
    backgroundColor: '#e8f5e9',
    borderRadius: 8,
    padding: 16,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#388e3c',
    minWidth: '80%',
  },
  resultLabel: {
    fontSize: 14,
    fontWeight: '600',
    color: '#1b5e20',
    marginBottom: 8,
  },
  resultValue: {
    fontSize: 16,
    fontWeight: '700',
    color: '#2e7d32',
    fontFamily: 'monospace',
    marginBottom: 8,
  },
  resultType: {
    fontSize: 12,
    color: '#558b2f',
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
  clearButton: {
    backgroundColor: '#f57c00',
    borderRadius: 6,
    paddingVertical: 10,
    paddingHorizontal: 16,
    marginTop: 8,
  },
  clearButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '600',
    textAlign: 'center',
  },
});
