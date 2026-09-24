import React from 'react';
import { Platform } from 'react-native';
import { apiClient } from '@services/api';
import { storageUtils } from '@services/storage';
import { useAppStore } from '@store/appStore';

const OFFLINE_QUEUE_STORAGE_KEY = 'offline-queue:v1';
const MAX_RETRY_COUNT = Platform.select({ ios: 5, android: 4, default: 3 });

export type OfflineQueueMethod = 'POST' | 'PUT' | 'DELETE';
export type OfflineQueueStatus = 'pending' | 'processing' | 'failed';

export interface OfflineQueueItem {
  id: string;
  method: OfflineQueueMethod;
  url: string;
  payload?: unknown;
  retryCount: number;
  status: OfflineQueueStatus;
  createdAt: string;
  updatedAt: string;
  lastError?: string;
}

export interface EnqueueOfflineRequest {
  method: OfflineQueueMethod;
  url: string;
  payload?: unknown;
}

export interface OfflineQueueState {
  items: OfflineQueueItem[];
  isProcessing: boolean;
  error?: string;
}

export interface UseOfflineQueueResult extends OfflineQueueState {
  enqueue: (request: EnqueueOfflineRequest) => OfflineQueueItem;
  remove: (id: string) => void;
  clear: () => void;
  retryFailed: () => Promise<void>;
  processQueue: () => Promise<void>;
}

function createQueueId(): string {
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function readQueue(): OfflineQueueItem[] {
  const value = storageUtils.getItem(OFFLINE_QUEUE_STORAGE_KEY);

  if (!value) {
    return [];
  }

  try {
    const parsed = JSON.parse(value);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    storageUtils.removeItem(OFFLINE_QUEUE_STORAGE_KEY);
    return [];
  }
}

function writeQueue(items: OfflineQueueItem[]): void {
  storageUtils.setItem(OFFLINE_QUEUE_STORAGE_KEY, JSON.stringify(items));
}

async function executeQueueItem(item: OfflineQueueItem): Promise<void> {
  if (item.method === 'POST') {
    await apiClient.post(item.url, item.payload);
    return;
  }

  if (item.method === 'PUT') {
    await apiClient.put(item.url, item.payload);
    return;
  }

  await apiClient.delete(item.url);
}

export async function processOfflineQueue(): Promise<OfflineQueueItem[]> {
  const { isOnline, setSyncStatus } = useAppStore.getState();
  const currentItems = readQueue();

  if (!isOnline || currentItems.length === 0) {
    return currentItems;
  }

  setSyncStatus(true);

  try {
    const remainingItems: OfflineQueueItem[] = [];

    for (const item of currentItems) {
      if (item.status === 'failed') {
        remainingItems.push(item);
        continue;
      }

      try {
        await executeQueueItem({ ...item, status: 'processing' });
      } catch (error) {
        const retryCount = item.retryCount + 1;
        const message = error instanceof Error ? error.message : 'Unable to sync queued request';

        remainingItems.push({
          ...item,
          retryCount,
          status: 'failed',
          lastError:
            retryCount >= (MAX_RETRY_COUNT ?? 3)
              ? `${message}. Maximum retry count reached. Use retry to attempt sync again.`
              : message,
          updatedAt: new Date().toISOString(),
        });
      }
    }

    writeQueue(remainingItems);
    return remainingItems;
  } finally {
    setSyncStatus(false);
  }
}

export function useOfflineQueue(): UseOfflineQueueResult {
  const isOnline = useAppStore(state => state.isOnline);
  const [state, setState] = React.useState<OfflineQueueState>({
    items: readQueue(),
    isProcessing: false,
  });

  const refresh = React.useCallback(() => {
    setState(current => ({ ...current, items: readQueue() }));
  }, []);

  const enqueue = React.useCallback((request: EnqueueOfflineRequest) => {
    const now = new Date().toISOString();
    const item: OfflineQueueItem = {
      id: createQueueId(),
      method: request.method,
      url: request.url,
      payload: request.payload,
      retryCount: 0,
      status: 'pending',
      createdAt: now,
      updatedAt: now,
    };
    const items = [...readQueue(), item];

    writeQueue(items);
    setState(current => ({ ...current, items, error: undefined }));

    return item;
  }, []);

  const remove = React.useCallback((id: string) => {
    const items = readQueue().filter(item => item.id !== id);

    writeQueue(items);
    setState(current => ({ ...current, items }));
  }, []);

  const clear = React.useCallback(() => {
    writeQueue([]);
    setState(current => ({ ...current, items: [], error: undefined }));
  }, []);

  const processQueue = React.useCallback(async () => {
    setState(current => ({ ...current, isProcessing: true, error: undefined }));

    try {
      const items = await processOfflineQueue();
      setState(current => ({ ...current, items, isProcessing: false }));
    } catch (error) {
      const message = error instanceof Error ? error.message : 'Offline queue sync failed';
      setState(current => ({ ...current, isProcessing: false, error: message }));
    }
  }, []);

  const retryFailed = React.useCallback(async () => {
    const items = readQueue().map(item =>
      item.status === 'failed' ? { ...item, status: 'pending' as const, updatedAt: new Date().toISOString() } : item
    );

    writeQueue(items);
    await processQueue();
  }, [processQueue]);

  React.useEffect(() => {
    refresh();
  }, [refresh]);

  React.useEffect(() => {
    if (isOnline) {
      processQueue();
    }
  }, [isOnline, processQueue]);

  return {
    ...state,
    enqueue,
    remove,
    clear,
    retryFailed,
    processQueue,
  };
}
