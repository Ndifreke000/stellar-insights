import { useCallback, useEffect, useState } from 'react';
import { useNetInfo } from '@react-native-community/netinfo';
import { Platform, GestureResponderEvent } from 'react-native';

interface TouchData {
  force: number;
  x: number;
  y: number;
  timestamp: number;
}

interface UseForceTouch {
  supported: boolean;
  loading: boolean;
  error: string | null;
  isOffline: boolean;
  touchData: TouchData | null;
  handlePress: (evt: GestureResponderEvent) => void;
}

export function useForceTouch(): UseForceTouch {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [touchData, setTouchData] = useState<TouchData | null>(null);
  const netInfo = useNetInfo();

  useEffect(() => {
    setIsOffline(!netInfo.isConnected || netInfo.isInternetReachable === false);
  }, [netInfo]);

  const handlePress = useCallback((evt: GestureResponderEvent) => {
    setLoading(true);
    setError(null);
    try {
      if (Platform.OS !== 'ios') {
        throw new Error('Force Touch is available on iOS 13.0 and later.');
      }
      const { force = 0 } = evt.nativeEvent;
      const pressure = Math.min(force, 1);
      setTouchData({
        force: pressure,
        x: evt.nativeEvent.locationX,
        y: evt.nativeEvent.locationY,
        timestamp: Date.now(),
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Force Touch detection failed.';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    supported: Platform.OS === 'ios',
    loading,
    error,
    isOffline,
    touchData,
    handlePress,
  };
}
