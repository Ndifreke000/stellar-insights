import { useState, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import { useMMKVStorage } from 'react-native-mmkv';

interface BarcodeResult {
  data: string;
  type?: string;
  timestamp: number;
}

interface BarcodeScannerState {
  isScanning: boolean;
  result: BarcodeResult | null;
  error: string | null;
  hasPermission: () => Promise<boolean>;
  requestPermission: () => Promise<boolean>;
  startScan: () => Promise<void>;
  stopScan: () => Promise<void>;
  resetResult: () => void;
}

const storage = require('react-native-mmkv').default;
const BARCODE_CACHE_KEY = 'barcode_scanner_cache';
const BARCODE_PERMISSIONS_KEY = 'barcode_permissions_granted';

export const useBarcodeScanner = (): BarcodeScannerState => {
  const [isScanning, setIsScanning] = useState(false);
  const [result, setResult] = useState<BarcodeResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const scanningTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const [permissionsGranted, setPermissionsGranted] = useMMKVStorage(BARCODE_PERMISSIONS_KEY, storage, false);

  const hasPermission = useCallback(async (): Promise<boolean> => {
    try {
      // On Android, we need camera permission
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const granted = await PermissionsAndroid.check(PermissionsAndroid.PERMISSIONS.CAMERA);
        return granted;
      }
      // On iOS, camera permission is checked at app level
      return true;
    } catch (err) {
      console.warn('Error checking permissions:', err);
      return false;
    }
  }, []);

  const requestPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const result = await PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.CAMERA, {
          title: 'Camera Permission',
          message: 'Barcode Scanner needs access to your camera',
          buttonNeutral: 'Ask Me Later',
          buttonNegative: 'Cancel',
          buttonPositive: 'OK',
        });
        const granted = result === PermissionsAndroid.RESULTS.GRANTED;
        setPermissionsGranted(granted);
        return granted;
      }
      // iOS permissions are handled by the system
      return true;
    } catch (err) {
      console.error('Error requesting permissions:', err);
      return false;
    }
  }, [setPermissionsGranted]);

  const startScan = useCallback(async (): Promise<void> => {
    try {
      setError(null);
      setIsScanning(true);

      const permission = await hasPermission();
      if (!permission) {
        const granted = await requestPermission();
        if (!granted) {
          throw new Error('Camera permission denied');
        }
      }

      // Simulate barcode scanning with timeout
      // In production, integrate with a barcode scanning library like react-native-camera
      if (scanningTimeoutRef.current) {
        clearTimeout(scanningTimeoutRef.current);
      }

      scanningTimeoutRef.current = setTimeout(() => {
        // Mock scan result for demonstration
        const mockBarcodes = [
          { data: '5901234123457', type: 'EAN-13' },
          { data: '123456789', type: 'CODE-128' },
          { data: '8006543210282', type: 'UPC-A' },
        ];
        const randomResult = mockBarcodes[Math.floor(Math.random() * mockBarcodes.length)];

        const barcodeResult: BarcodeResult = {
          ...randomResult,
          timestamp: Date.now(),
        };

        setResult(barcodeResult);
        setIsScanning(false);

        // Cache result for offline support
        try {
          const cached = storage.getString(BARCODE_CACHE_KEY);
          const cachedResults = cached ? JSON.parse(cached) : [];
          cachedResults.push(barcodeResult);
          // Keep only last 100 scans
          if (cachedResults.length > 100) {
            cachedResults.shift();
          }
          storage.set(BARCODE_CACHE_KEY, JSON.stringify(cachedResults));
        } catch (cacheErr) {
          console.warn('Failed to cache barcode result:', cacheErr);
        }
      }, 3000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start barcode scan';
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
    setResult(null);
    setError(null);
  }, []);

  return {
    isScanning,
    result,
    error,
    hasPermission,
    requestPermission,
    startScan,
    stopScan,
    resetResult,
  };
};
