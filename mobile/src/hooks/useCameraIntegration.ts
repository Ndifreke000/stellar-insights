import { useState, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import { useMMKVBoolean } from 'react-native-mmkv';

export type CameraMode = 'qr' | 'document' | 'photo';

export interface QRScanResult {
  data: string;
  type: string;
  timestamp: number;
}

export interface CapturedPhoto {
  uri: string;
  width: number;
  height: number;
  timestamp: number;
}

interface CameraIntegrationState {
  isActive: boolean;
  mode: CameraMode;
  qrResult: QRScanResult | null;
  capturedPhoto: CapturedPhoto | null;
  error: string | null;
  hasPermission: () => Promise<boolean>;
  requestPermission: () => Promise<boolean>;
  startCamera: (mode?: CameraMode) => Promise<void>;
  stopCamera: () => void;
  capturePhoto: () => Promise<void>;
  resetResult: () => void;
  setMode: (mode: CameraMode) => void;
}

const CAMERA_CACHE_KEY = 'camera_integration_cache';
const CAMERA_PERMISSIONS_KEY = 'camera_permissions_granted';
const storage = require('react-native-mmkv').default;

export const useCameraIntegration = (): CameraIntegrationState => {
  const [isActive, setIsActive] = useState(false);
  const [mode, setMode] = useState<CameraMode>('qr');
  const [qrResult, setQrResult] = useState<QRScanResult | null>(null);
  const [capturedPhoto, setCapturedPhoto] = useState<CapturedPhoto | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [, setPermissionsGranted] = useMMKVBoolean(CAMERA_PERMISSIONS_KEY, storage);
  const scanTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const hasPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        return await PermissionsAndroid.check(PermissionsAndroid.PERMISSIONS.CAMERA);
      }
      return true;
    } catch (err) {
      console.warn('Error checking camera permission:', err);
      return false;
    }
  }, []);

  const requestPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const result = await PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.CAMERA, {
          title: 'Camera Permission',
          message: 'PayRaider needs camera access for QR code scanning and document capture',
          buttonNeutral: 'Ask Me Later',
          buttonNegative: 'Cancel',
          buttonPositive: 'OK',
        });
        const granted = result === PermissionsAndroid.RESULTS.GRANTED;
        setPermissionsGranted(granted);
        return granted;
      }
      return true;
    } catch (err) {
      console.error('Error requesting camera permission:', err);
      return false;
    }
  }, [setPermissionsGranted]);

  const startCamera = useCallback(async (cameraMode?: CameraMode): Promise<void> => {
    try {
      setError(null);
      if (cameraMode) setMode(cameraMode);

      const permission = await hasPermission();
      if (!permission) {
        const granted = await requestPermission();
        if (!granted) {
          throw new Error('Camera permission denied');
        }
      }

      setIsActive(true);

      // QR mode: auto-simulate a scan after a short delay
      const activeMode = cameraMode ?? mode;
      if (activeMode === 'qr') {
        if (scanTimeoutRef.current) clearTimeout(scanTimeoutRef.current);
        scanTimeoutRef.current = setTimeout(() => {
          const mockQRData = [
            { data: 'stellar:GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFL4NAT4AQH3ZLLFLA5', type: 'QR' },
            { data: 'stellar:GDQOE23CFSUMSVQK4Y5JHPPYK73VYCNHZHA7ENKCV37P6SUEO6XQBKPP', type: 'QR' },
            { data: 'web+stellar:pay?destination=GCEZWKCA5VLDNRLN3RPRJMRZOX3Z6G5CHCGQKPG3EQ7AQVNKAW6AT2BW&amount=100', type: 'QR' },
          ];
          const mock = mockQRData[Math.floor(Math.random() * mockQRData.length)];
          const result: QRScanResult = { ...mock, timestamp: Date.now() };
          setQrResult(result);
          setIsActive(false);
          cacheResult('qr', result);
        }, 2500);
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to start camera';
      setError(msg);
      setIsActive(false);
    }
  }, [hasPermission, requestPermission, mode]);

  const capturePhoto = useCallback(async (): Promise<void> => {
    try {
      setError(null);
      // Simulate photo capture for document mode
      const photo: CapturedPhoto = {
        uri: `file://camera_capture_${Date.now()}.jpg`,
        width: 1920,
        height: 1080,
        timestamp: Date.now(),
      };
      setCapturedPhoto(photo);
      setIsActive(false);
      cacheResult('photo', photo);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to capture photo';
      setError(msg);
    }
  }, []);

  const stopCamera = useCallback(() => {
    if (scanTimeoutRef.current) {
      clearTimeout(scanTimeoutRef.current);
      scanTimeoutRef.current = null;
    }
    setIsActive(false);
  }, []);

  const resetResult = useCallback(() => {
    setQrResult(null);
    setCapturedPhoto(null);
    setError(null);
  }, []);

  return {
    isActive,
    mode,
    qrResult,
    capturedPhoto,
    error,
    hasPermission,
    requestPermission,
    startCamera,
    stopCamera,
    capturePhoto,
    resetResult,
    setMode,
  };
};

function cacheResult(type: string, data: unknown) {
  try {
    const raw = storage.getString(CAMERA_CACHE_KEY);
    const cached: unknown[] = raw ? JSON.parse(raw) : [];
    cached.push({ type, data });
    if (cached.length > 50) cached.shift();
    storage.set(CAMERA_CACHE_KEY, JSON.stringify(cached));
  } catch {
    console.warn('Failed to cache camera result');
  }
}
