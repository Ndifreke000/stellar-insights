import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ActivityIndicator, TouchableOpacity, AccessibilityRole } from 'react-native';
import { useFaceRecognition } from './useFaceRecognition';

export const FaceRecognitionComponent = () => {
  const {
    isScanning,
    isRecognized,
    error,
    startScan,
    stopScan,
    resetResult,
    hasPermission,
    requestPermission,
    isCameraAvailable,
    confidenceScore,
  } = useFaceRecognition();

  const [isInitialized, setIsInitialized] = useState(false);
  const [cameraAvailable, setCameraAvailable] = useState(false);

  useEffect(() => {
    const initialize = async () => {
      try {
        const camAvail = await isCameraAvailable();
        setCameraAvailable(camAvail);

        if (camAvail) {
          const hasPermissionResult = await hasPermission();
          if (!hasPermissionResult) {
            await requestPermission();
          }
        }
      } catch (err) {
        console.warn('Failed to initialize face recognition:', err);
      }
      setIsInitialized(true);
    };

    initialize();
  }, []);

  const handleStartScan = async () => {
    try {
      await startScan();
    } catch (err) {
      console.error('Failed to start face recognition:', err);
    }
  };

  if (!isInitialized) {
    return (
      <View style={styles.container} accessibilityRole="status">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing face recognition">
          Initializing Face Recognition...
        </Text>
      </View>
    );
  }

  if (!cameraAvailable) {
    return (
      <View style={styles.container} accessibilityRole="alert">
        <View style={styles.errorContainer}>
          <Text style={styles.errorText}>Camera not available on this device</Text>
        </View>
      </View>
    );
  }

  return (
    <View style={styles.container} accessibilityRole="main">
      <View
        style={styles.header}
        accessible={true}
        accessibilityLabel="Face Recognition Interface"
        accessibilityRole="header"
      >
        <Text style={styles.title}>Face Recognition</Text>
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

        {isRecognized && (
          <View
            style={styles.successContainer}
            accessible={true}
            accessibilityRole="status"
            accessibilityLabel={`Face recognized with ${(confidenceScore * 100).toFixed(0)}% confidence`}
          >
            <Text style={styles.successText}>✓ Face Recognized</Text>
            <Text style={styles.confidenceText}>
              Confidence: {(confidenceScore * 100).toFixed(1)}%
            </Text>
            <TouchableOpacity
              style={styles.resetButton}
              onPress={resetResult}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Reset recognition"
            >
              <Text style={styles.resetButtonText}>Reset</Text>
            </TouchableOpacity>
          </View>
        )}

        {!isScanning && !isRecognized && (
          <TouchableOpacity
            style={styles.scanButton}
            onPress={handleStartScan}
            disabled={isScanning}
            accessible={true}
            accessibilityRole="button"
            accessibilityLabel="Start face recognition"
            accessibilityHint="Tap to authenticate with face recognition"
          >
            {isScanning ? (
              <ActivityIndicator size="small" color="#fff" />
            ) : (
              <Text style={styles.scanButtonText}>Recognize Face</Text>
            )}
          </TouchableOpacity>
        )}

        {isScanning && (
          <View style={styles.scanningContainer}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.scanningText} accessibilityLabel="Position your face in the frame">
              Position your face in the frame
            </Text>
            <TouchableOpacity
              style={styles.stopButton}
              onPress={stopScan}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Cancel recognition"
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
    marginBottom: 8,
  },
  confidenceText: {
    color: '#558b2f',
    fontSize: 14,
    fontWeight: '600',
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
