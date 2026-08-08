import { useState, useEffect, useCallback, useRef } from 'react';
import { Platform, AppState, AppStateStatus } from 'react-native';
import type {
  BackgroundLocationConfig,
  BackgroundLocationState,
  LocationCoordinates,
  LocationPermissionStatus,
} from '../features/background_location/types';

const DEFAULT_CONFIG: BackgroundLocationConfig = {
  distanceFilter: 10,
  interval: 30000,
  showsBackgroundLocationIndicator: true,
  notificationTitle: 'PayRaider',
  notificationBody: 'Tracking location in background',
};

const INITIAL_STATE: BackgroundLocationState = {
  permissionStatus: 'not_requested',
  isTracking: false,
  lastLocation: null,
  locationHistory: [],
  error: null,
  isLoading: false,
};

export function useBackgroundLocation(config: Partial<BackgroundLocationConfig> = {}) {
  const mergedConfig: BackgroundLocationConfig = { ...DEFAULT_CONFIG, ...config };
  const [state, setState] = useState<BackgroundLocationState>(INITIAL_STATE);
  const appState = useRef<AppStateStatus>(AppState.currentState);
  const taskRef = useRef<{ stop: () => void } | null>(null);

  useEffect(() => {
    const sub = AppState.addEventListener('change', next => { appState.current = next; });
    return () => sub.remove();
  }, []);

  const requestPermission = useCallback(async (): Promise<LocationPermissionStatus> => {
    setState(s => ({ ...s, isLoading: true, error: null }));
    try {
      // Wire to expo-location or react-native-permissions:
      // const { status: fg } = await Location.requestForegroundPermissionsAsync();
      // const { status: bg } = await Location.requestBackgroundPermissionsAsync();
      const status: LocationPermissionStatus = 'background_granted';
      setState(s => ({ ...s, permissionStatus: status, isLoading: false }));
      return status;
    } catch (err: any) {
      const error = err?.message ?? 'Permission request failed';
      setState(s => ({ ...s, error, isLoading: false, permissionStatus: 'denied' }));
      return 'denied';
    }
  }, []);

  const startTracking = useCallback(async () => {
    if (state.isTracking) return;
    let permission = state.permissionStatus;
    if (permission !== 'background_granted') {
      permission = await requestPermission();
    }
    if (permission === 'denied' || permission === 'unavailable') return;
    setState(s => ({ ...s, isLoading: true, error: null }));
    try {
      // Wire to expo-location startLocationUpdatesAsync or rn-background-geolocation start()
      setState(s => ({ ...s, isTracking: true, isLoading: false }));
    } catch (err: any) {
      setState(s => ({ ...s, error: err?.message ?? 'Failed to start background location', isLoading: false }));
    }
  }, [state.isTracking, state.permissionStatus, mergedConfig, requestPermission]);

  const stopTracking = useCallback(async () => {
    if (!state.isTracking) return;
    try {
      // Wire to expo-location stopLocationUpdatesAsync or BackgroundGeolocation.stop()
      taskRef.current?.stop();
      taskRef.current = null;
      setState(s => ({ ...s, isTracking: false }));
    } catch (err: any) {
      setState(s => ({ ...s, error: err?.message ?? 'Failed to stop tracking' }));
    }
  }, [state.isTracking]);

  const onLocationUpdate = useCallback((coords: LocationCoordinates) => {
    setState(s => ({
      ...s,
      lastLocation: coords,
      locationHistory: [...s.locationHistory.slice(-99), coords],
    }));
  }, []);

  useEffect(() => {
    return () => { taskRef.current?.stop(); };
  }, []);

  const clearHistory = useCallback(() => {
    setState(s => ({ ...s, locationHistory: [], lastLocation: null }));
  }, []);

  return {
    ...state,
    requestPermission,
    startTracking,
    stopTracking,
    onLocationUpdate,
    clearHistory,
    config: mergedConfig,
  };
}
