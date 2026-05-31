import { useCallback, useEffect, useState } from 'react';
import { useNetInfo } from '@react-native-community/netinfo';
import { Platform } from 'react-native';

interface HandoffActivity {
  type: string;
  userInfo: Record<string, unknown>;
  needsSave: boolean;
}

interface UseHandoffSupport {
  supported: boolean;
  loading: boolean;
  error: string | null;
  isOffline: boolean;
  activity: HandoffActivity | null;
  continueActivity: (activity: HandoffActivity) => Promise<void>;
}

export function useHandoffSupport(): UseHandoffSupport {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [activity, setActivity] = useState<HandoffActivity | null>(null);
  const netInfo = useNetInfo();

  useEffect(() => {
    setIsOffline(!netInfo.isConnected || netInfo.isInternetReachable === false);
  }, [netInfo]);

  const continueActivity = useCallback(async (newActivity: HandoffActivity) => {
    setLoading(true);
    setError(null);
    try {
      if (Platform.OS !== 'ios') {
        throw new Error('Handoff is available on iOS 9.0 and later.');
      }
      setActivity(newActivity);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Handoff continuation failed.';
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
    activity,
    continueActivity,
  };
}
