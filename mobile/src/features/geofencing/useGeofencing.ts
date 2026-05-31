import { useCallback, useEffect, useState } from 'react';
import { useNetInfo } from '@react-native-community/netinfo';
import { Platform, PermissionsAndroid } from 'react-native';

interface Geofence {
  id: string;
  latitude: number;
  longitude: number;
  radius: number;
  name: string;
}

interface GeofenceEvent {
  fenceId: string;
  type: 'enter' | 'exit';
  timestamp: number;
}

interface UseGeofencing {
  supported: boolean;
  loading: boolean;
  error: string | null;
  isOffline: boolean;
  fences: Geofence[];
  events: GeofenceEvent[];
  addFence: (fence: Geofence) => Promise<void>;
  removeFence: (id: string) => Promise<void>;
}

export function useGeofencing(): UseGeofencing {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [fences, setFences] = useState<Geofence[]>([]);
  const [events, setEvents] = useState<GeofenceEvent[]>([]);
  const netInfo = useNetInfo();

  useEffect(() => {
    setIsOffline(!netInfo.isConnected || netInfo.isInternetReachable === false);
  }, [netInfo]);

  const addFence = useCallback(async (fence: Geofence) => {
    setLoading(true);
    setError(null);
    try {
      if (Platform.OS === 'android') {
        const granted = await PermissionsAndroid.request(
          PermissionsAndroid.PERMISSIONS.ACCESS_FINE_LOCATION
        );
        if (granted !== PermissionsAndroid.RESULTS.GRANTED) {
          throw new Error('Location permission denied.');
        }
      }
      setFences(prev => [...prev, fence]);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to add geofence.';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const removeFence = useCallback(async (id: string) => {
    setLoading(true);
    setError(null);
    try {
      setFences(prev => prev.filter(f => f.id !== id));
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to remove geofence.';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  return {
    supported: Platform.OS === 'ios' || Platform.OS === 'android',
    loading,
    error,
    isOffline,
    fences,
    events,
    addFence,
    removeFence,
  };
}
