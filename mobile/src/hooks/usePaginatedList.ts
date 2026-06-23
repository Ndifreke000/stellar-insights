import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useRef, useState } from 'react';

export interface CursorPage<T> {
  items: T[];
  next_cursor: string | null;
  total: number;
}

interface UsePaginatedListOptions<T> {
  cacheKey: string;
  fetchPage: (cursor: string | null) => Promise<CursorPage<T>>;
  mockData: () => CursorPage<T>;
}

export interface UsePaginatedListReturn<T> {
  items: T[];
  total: number;
  loading: boolean;
  loadingMore: boolean;
  hasMore: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  isFromCache: boolean;
  refresh: () => Promise<void>;
  loadMore: () => Promise<void>;
}

export function usePaginatedList<T>({
  cacheKey,
  fetchPage,
  mockData,
}: UsePaginatedListOptions<T>): UsePaginatedListReturn<T> {
  const [items, setItems] = useState<T[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [isFromCache, setIsFromCache] = useState(false);

  const nextCursorRef = useRef<string | null>(null);

  const checkOffline = async (): Promise<boolean> => {
    try {
      const state = await NetInfo.fetch();
      return !(state.isConnected && state.isInternetReachable !== false);
    } catch {
      return true;
    }
  };

  const readCache = async (): Promise<CursorPage<T> | null> => {
    try {
      const cached = await AsyncStorage.getItem(cacheKey);
      return cached ? (JSON.parse(cached) as CursorPage<T>) : null;
    } catch {
      return null;
    }
  };

  const writeCache = async (page: CursorPage<T>): Promise<void> => {
    try {
      await AsyncStorage.setItem(cacheKey, JSON.stringify(page));
    } catch {
      // Best-effort cache write.
    }
  };

  const applyPage = (page: CursorPage<T>, append: boolean) => {
    setItems(prev => (append ? [...prev, ...page.items] : page.items));
    setTotal(page.total);
    nextCursorRef.current = page.next_cursor;
    setHasMore(page.next_cursor !== null);
  };

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    setWarning(null);
    setIsFromCache(false);
    nextCursorRef.current = null;

    const offline = await checkOffline();
    setIsOffline(offline);

    try {
      if (offline) {
        const cached = await readCache();
        if (cached) {
          applyPage(cached, false);
          setIsFromCache(true);
          setWarning('Offline — showing saved data.');
        } else {
          applyPage(mockData(), false);
          setWarning('Offline — no saved data. Showing sample data.');
        }
        return;
      }

      try {
        const page = await fetchPage(null);
        applyPage(page, false);
        await writeCache(page);
      } catch {
        const cached = await readCache();
        if (cached) {
          applyPage(cached, false);
          setIsFromCache(true);
          setWarning('Live data unavailable. Showing saved data.');
        } else {
          applyPage(mockData(), false);
          setWarning('Live data unavailable. Showing sample data.');
        }
      }
    } catch {
      setError('Failed to load data.');
      setItems([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [cacheKey, fetchPage, mockData]); // eslint-disable-line react-hooks/exhaustive-deps

  const loadMore = useCallback(async () => {
    if (loadingMore || !nextCursorRef.current) return;

    setLoadingMore(true);
    try {
      const page = await fetchPage(nextCursorRef.current);
      applyPage(page, true);
    } catch {
      // Non-fatal: user can retry by scrolling again.
    } finally {
      setLoadingMore(false);
    }
  }, [loadingMore, fetchPage]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return {
    items,
    total,
    loading,
    loadingMore,
    hasMore,
    error,
    warning,
    isOffline,
    isFromCache,
    refresh,
    loadMore,
  };
}
