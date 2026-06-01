import { useState, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import { useMMKVStorage } from 'react-native-mmkv';

interface FingerprintScannerState {
  isScanning: boolean;
  isAuthenticated: boolean;
  error: string | null;
  hasPermission: () => Promise<boolean>;
  requestPermission: () => Promise<boolean>;
  startScan: () => Promise<void>;
  stopScan: () => Promise<void>;
  resetResult: () => void;
  isBiometricAvailable: () => Promise<boolean>;
}

const storage = require('react-native-mmkv').default;
const FINGERPRINT_CACHE_KEY = 'fingerprint_auth_cache';
const FINGERPRINT_PERMISSIONS_KEY = 'fingerprint_permissions_granted';

export const useFingerprintScanner = (): FingerprintScannerState => {
  const [isScanning, setIsScanning] = useState(false);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const scanningTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const [permissionsGranted, setPermissionsGranted] = useMMKVStorage(FINGERPRINT_PERMISSIONS_KEY, storage, false);

  const isBiometricAvailable = useCallback(async (): Promise<boolean> => {
    try {
      const RNBiometrics = require('react-native-biometrics');
      const compatible = await RNBiometrics.isSensorAvailable();
      return compatible;
    } catch (err) {
      console.warn('Biometric not available:', err);
      return false;
    }
  }, []);

  const hasPermission = useCallback(async (): Promise<boolean> => {
    try {
      // Biometric permissions are granted when device has biometric capability
      return await isBiometricAvailable();
    } catch (err) {
      console.warn('Error checking permissions:', err);
      return false;
    }
  }, [isBiometricAvailable]);

  const requestPermission = useCallback(async (): Promise<boolean> => {
    try {
      const available = await isBiometricAvailable();
      setPermissionsGranted(available);
      return available;
    } catch (err) {
      console.error('Error requesting permissions:', err);
      return false;
    }
  }, [isBiometricAvailable, setPermissionsGranted]);

  const startScan = useCallback(async (): Promise<void> => {
    try {
      setError(null);
      setIsScanning(true);

      const permission = await hasPermission();
      if (!permission) {
        const granted = await requestPermission();
        if (!granted) {
          throw new Error('Biometric authentication not available');
        }
      }

      // Simulate fingerprint scanning with timeout
      if (scanningTimeoutRef.current) {
        clearTimeout(scanningTimeoutRef.current);
      }

      scanningTimeoutRef.current = setTimeout(() => {
        // Simulate 80% success rate
        const success = Math.random() > 0.2;

        if (success) {
          setIsAuthenticated(true);
          setIsScanning(false);

          // Cache authentication for offline support
          try {
            const cacheData = {
              authenticated: true,
              timestamp: Date.now(),
              userId: 'user_' + Math.random().toString(36).substr(2, 9),
            };
            storage.set(FINGERPRINT_CACHE_KEY, JSON.stringify(cacheData));
          } catch (cacheErr) {
            console.warn('Failed to cache fingerprint auth:', cacheErr);
          }
        } else {
          setError('Fingerprint not recognized. Please try again.');
          setIsScanning(false);
        }
      }, 2500);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start fingerprint scan';
      setError(errorMessage);
      setIsScanning(false);
    }
  }, [hasPermission, requestPermission]);

  const stopScan = useCallback(async (): Promise<void> => {
    if (scanningTimeoutRef.current) {
      clearTimeout(scanningTimeoutRef.current);
      scanningTimeoutRef.current = null;
    }
    setIsScanning(false);
  }, []);

  const resetResult = useCallback((): void => {
    setIsAuthenticated(false);
    setError(null);
  }, []);

  return {
    isScanning,
    isAuthenticated,
    error,
    hasPermission,
    requestPermission,
    startScan,
    stopScan,
    resetResult,
    isBiometricAvailable,
  };
};
