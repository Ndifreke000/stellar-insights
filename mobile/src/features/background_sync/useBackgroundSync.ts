import { useCallback, useEffect, useState } from 'react';
import { useNetInfo } from '@react-native-community/netinfo';
import { Platform, AppState } from 'react-native';

interface SyncTask {
  id: string;
  type: string;
  data: Record<string, unknown>;
  status: 'pending' | 'syncing' | 'completed' | 'failed';
}

interface UseBackgroundSync {
  supported: boolean;
  loading: boolean;
  error: string | null;
  isOffline: boolean;
  tasks: SyncTask[];
  queueTask: (task: Omit<SyncTask, 'id' | 'status'>) => Promise<void>;
  syncNow: () => Promise<void>;
}

export function useBackgroundSync(): UseBackgroundSync {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [tasks, setTasks] = useState<SyncTask[]>([]);
  const netInfo = useNetInfo();

  useEffect(() => {
    setIsOffline(!netInfo.isConnected || netInfo.isInternetReachable === false);
  }, [netInfo]);

  useEffect(() => {
    const handleAppStateChange = (state: string) => {
      if (state === 'background') {
        // Schedule background sync when app goes to background
        syncOfflineChanges();
      }
    };
    const subscription = AppState.addEventListener('change', handleAppStateChange);
    return () => subscription.remove();
  }, []);

  const queueTask = useCallback(async (task: Omit<SyncTask, 'id' | 'status'>) => {
    setLoading(true);
    setError(null);
    try {
      const newTask: SyncTask = {
        id: `sync_${Date.now()}`,
        status: 'pending',
        ...task,
      };
      setTasks(prev => [...prev, newTask]);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Failed to queue task.';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, []);

  const syncOfflineChanges = useCallback(async () => {
    const offlineTasks = tasks.filter(t => t.status === 'pending');
    if (offlineTasks.length === 0) return;

    setTasks(prev => prev.map(t => (t.status === 'pending' ? { ...t, status: 'syncing' } : t)));

    try {
      // Simulate background sync
      await new Promise(resolve => setTimeout(resolve, 1000));
      setTasks(prev => prev.map(t => (t.status === 'syncing' ? { ...t, status: 'completed' } : t)));
    } catch (err) {
      setTasks(prev => prev.map(t => (t.status === 'syncing' ? { ...t, status: 'failed' } : t)));
    }
  }, [tasks]);

  const syncNow = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      await syncOfflineChanges();
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Sync failed.';
      setError(message);
    } finally {
      setLoading(false);
    }
  }, [syncOfflineChanges]);

  return {
    supported: Platform.OS === 'ios' || Platform.OS === 'android',
    loading,
    error,
    isOffline,
    tasks,
    queueTask,
    syncNow,
  };
}
