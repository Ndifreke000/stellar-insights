import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  TouchableOpacity,
  ActivityIndicator,
  Image,
} from 'react-native';
import { useCameraIntegration, CameraMode } from '../hooks/useCameraIntegration';

export const CameraIntegrationComponent = () => {
  const {
    isActive,
    mode,
    qrResult,
    capturedPhoto,
    error,
    startCamera,
    stopCamera,
    capturePhoto,
    resetResult,
    setMode,
    hasPermission,
    requestPermission,
  } = useCameraIntegration();

  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    (async () => {
      const permitted = await hasPermission();
      if (!permitted) await requestPermission();
      setIsInitialized(true);
    })().catch(() => setIsInitialized(true));

    return () => {
      if (isActive) stopCamera();
    };
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  if (!isInitialized) {
    return (
      <View style={styles.container} accessibilityLiveRegion="polite">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing camera">
          Initializing Camera...
        </Text>
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <View style={styles.header} accessible accessibilityRole="header">
        <Text style={styles.title}>Camera Integration</Text>
      </View>

      {/* Mode selector */}
      <View style={styles.modeRow} accessibilityRole="tablist">
        {(['qr', 'document', 'photo'] as CameraMode[]).map(m => (
          <TouchableOpacity
            key={m}
            style={[styles.modeButton, mode === m && styles.modeButtonActive]}
            onPress={() => setMode(m)}
            accessible
            accessibilityRole="tab"
            accessibilityState={{ selected: mode === m }}
            accessibilityLabel={`${m} mode`}
          >
            <Text style={[styles.modeText, mode === m && styles.modeTextActive]}>
              {m === 'qr' ? 'QR Scan' : m === 'document' ? 'Document' : 'Photo'}
            </Text>
          </TouchableOpacity>
        ))}
      </View>

      <View style={styles.content}>
        {error && (
          <View style={styles.errorContainer} accessible accessibilityRole="alert" accessibilityLabel={`Error: ${error}`}>
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        {/* QR result */}
        {qrResult && (
          <View style={styles.resultContainer} accessible accessibilityLiveRegion="polite" accessibilityLabel={`QR code scanned: ${qrResult.data}`}>
            <Text style={styles.resultLabel}>QR Code Scanned</Text>
            <Text style={styles.resultValue} selectable>{qrResult.data}</Text>
            <Text style={styles.resultMeta}>Type: {qrResult.type}</Text>
            <TouchableOpacity style={styles.clearButton} onPress={resetResult} accessible accessibilityRole="button" accessibilityLabel="Clear result">
              <Text style={styles.clearButtonText}>Clear</Text>
            </TouchableOpacity>
          </View>
        )}

        {/* Captured photo result */}
        {capturedPhoto && (
          <View style={styles.resultContainer} accessible accessibilityLiveRegion="polite" accessibilityLabel="Photo captured">
            <Text style={styles.resultLabel}>Photo Captured</Text>
            <Text style={styles.resultValue} selectable>{capturedPhoto.uri}</Text>
            <Text style={styles.resultMeta}>{capturedPhoto.width} × {capturedPhoto.height}</Text>
            <TouchableOpacity style={styles.clearButton} onPress={resetResult} accessible accessibilityRole="button" accessibilityLabel="Clear captured photo">
              <Text style={styles.clearButtonText}>Clear</Text>
            </TouchableOpacity>
          </View>
        )}

        {/* Active scanning / capture view */}
        {isActive && (
          <View style={styles.activeContainer} accessible accessibilityLiveRegion="polite" accessibilityLabel={mode === 'qr' ? 'Scanning for QR code' : 'Camera is ready'}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.activeText}>
              {mode === 'qr' ? 'Scanning for QR code...' : 'Camera ready'}
            </Text>
            {mode !== 'qr' && (
              <TouchableOpacity
                style={styles.captureButton}
                onPress={capturePhoto}
                accessible
                accessibilityRole="button"
                accessibilityLabel="Capture photo"
              >
                <Text style={styles.captureButtonText}>Capture</Text>
              </TouchableOpacity>
            )}
            <TouchableOpacity
              style={styles.stopButton}
              onPress={stopCamera}
              accessible
              accessibilityRole="button"
              accessibilityLabel="Stop camera"
            >
              <Text style={styles.stopButtonText}>Stop</Text>
            </TouchableOpacity>
          </View>
        )}

        {/* Start button */}
        {!isActive && !qrResult && !capturedPhoto && (
          <TouchableOpacity
            style={styles.startButton}
            onPress={() => startCamera(mode)}
            accessible
            accessibilityRole="button"
            accessibilityLabel={`Start ${mode === 'qr' ? 'QR scan' : mode + ' capture'}`}
            accessibilityHint={mode === 'qr' ? 'Scans a QR code with the camera' : 'Opens camera for capture'}
          >
            <Text style={styles.startButtonText}>
              {mode === 'qr' ? 'Scan QR Code' : `Open ${mode === 'document' ? 'Document' : 'Photo'} Camera`}
            </Text>
          </TouchableOpacity>
        )}
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  container: { flex: 1, backgroundColor: '#f5f5f5' },
  header: {
    paddingVertical: 16,
    paddingHorizontal: 16,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  title: { fontSize: 24, fontWeight: '600', color: '#333' },
  modeRow: {
    flexDirection: 'row',
    padding: 12,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
    gap: 8,
  },
  modeButton: {
    flex: 1,
    paddingVertical: 8,
    borderRadius: 8,
    backgroundColor: '#f0f0f0',
    alignItems: 'center',
  },
  modeButtonActive: { backgroundColor: '#007AFF' },
  modeText: { fontSize: 13, fontWeight: '500', color: '#555' },
  modeTextActive: { color: '#fff' },
  content: { flex: 1, justifyContent: 'center', alignItems: 'center', padding: 16 },
  loadingText: { marginTop: 12, fontSize: 16, color: '#666' },
  errorContainer: {
    backgroundColor: '#ffebee',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#d32f2f',
    width: '100%',
  },
  errorText: { color: '#d32f2f', fontSize: 14, fontWeight: '500' },
  resultContainer: {
    backgroundColor: '#e8f5e9',
    borderRadius: 8,
    padding: 16,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#388e3c',
    width: '100%',
  },
  resultLabel: { fontSize: 14, fontWeight: '600', color: '#1b5e20', marginBottom: 8 },
  resultValue: { fontSize: 13, fontWeight: '700', color: '#2e7d32', fontFamily: 'monospace', marginBottom: 4 },
  resultMeta: { fontSize: 12, color: '#558b2f', marginBottom: 12 },
  clearButton: {
    backgroundColor: '#f57c00',
    borderRadius: 6,
    paddingVertical: 8,
    paddingHorizontal: 16,
    alignSelf: 'flex-start',
  },
  clearButtonText: { color: '#fff', fontSize: 14, fontWeight: '600' },
  activeContainer: { alignItems: 'center', gap: 16 },
  activeText: { fontSize: 16, color: '#666', marginTop: 8 },
  captureButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 14,
    paddingHorizontal: 40,
  },
  captureButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  stopButton: {
    backgroundColor: '#d32f2f',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 32,
  },
  stopButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
  startButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 14,
    paddingHorizontal: 32,
    minWidth: '70%',
    alignItems: 'center',
  },
  startButtonText: { color: '#fff', fontSize: 16, fontWeight: '600' },
});
