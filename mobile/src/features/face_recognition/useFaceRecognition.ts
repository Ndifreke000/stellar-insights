import { useState, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import { useMMKVStorage } from 'react-native-mmkv';

interface FaceRecognitionState {
  isScanning: boolean;
  isRecognized: boolean;
  error: string | null;
  confidenceScore: number;
  hasPermission: () => Promise<boolean>;
  requestPermission: () => Promise<boolean>;
  startScan: () => Promise<void>;
  stopScan: () => Promise<void>;
  resetResult: () => void;
  isCameraAvailable: () => Promise<boolean>;
}

const storage = require('react-native-mmkv').default;
const FACE_RECOGNITION_CACHE_KEY = 'face_recognition_cache';
const FACE_PERMISSIONS_KEY = 'face_permissions_granted';

export const useFaceRecognition = (): FaceRecognitionState => {
  const [isScanning, setIsScanning] = useState(false);
  const [isRecognized, setIsRecognized] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confidenceScore, setConfidenceScore] = useState(0);
  const scanningTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const [permissionsGranted, setPermissionsGranted] = useMMKVStorage(FACE_PERMISSIONS_KEY, storage, false);

  const isCameraAvailable = useCallback(async (): Promise<boolean> => {
    try {
      // Check if device has camera capability
      // In production, use react-native-camera or ml-kit
      return true; // Assume most devices have cameras
    } catch (err) {
      console.warn('Camera not available:', err);
      return false;
    }
  }, []);

  const hasPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const granted = await PermissionsAndroid.check(PermissionsAndroid.PERMISSIONS.CAMERA);
        return granted;
      }
      // iOS camera permission is checked at system level
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
          message: 'Face Recognition needs access to your camera',
          buttonNeutral: 'Ask Me Later',
          buttonNegative: 'Cancel',
          buttonPositive: 'OK',
        });
        const granted = result === PermissionsAndroid.RESULTS.GRANTED;
        setPermissionsGranted(granted);
        return granted;
      }
      // iOS permissions handled by system
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
      setConfidenceScore(0);

      const permission = await hasPermission();
      if (!permission) {
        const granted = await requestPermission();
        if (!granted) {
          throw new Error('Camera permission denied');
        }
      }

      // Simulate face recognition scanning with timeout
      // In production, integrate with ML Kit Face Detection or similar
      if (scanningTimeoutRef.current) {
        clearTimeout(scanningTimeoutRef.current);
      }

      scanningTimeoutRef.current = setTimeout(() => {
        // Simulate 75% success rate with random confidence
        const success = Math.random() > 0.25;

        if (success) {
          const confidence = 0.85 + Math.random() * 0.15; // 85-100% confidence
          setConfidenceScore(confidence);
          setIsRecognized(true);
          setIsScanning(false);

          // Cache recognition data for offline support
          try {
            const cacheData = {
              recognized: true,
              timestamp: Date.now(),
              confidence,
              userId: 'user_' + Math.random().toString(36).substr(2, 9),
            };
            storage.set(FACE_RECOGNITION_CACHE_KEY, JSON.stringify(cacheData));
          } catch (cacheErr) {
            console.warn('Failed to cache face recognition:', cacheErr);
          }
        } else {
          setError('Face not recognized. Please try again or ensure good lighting.');
          setIsScanning(false);
        }
      }, 3500);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start face recognition';
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
    setIsRecognized(false);
    setError(null);
    setConfidenceScore(0);
  }, []);

  return {
    isScanning,
    isRecognized,
    error,
    confidenceScore,
    hasPermission,
    requestPermission,
    startScan,
    stopScan,
    resetResult,
    isCameraAvailable,
  };
};
