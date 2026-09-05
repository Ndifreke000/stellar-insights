import React from 'react';
import { Platform } from 'react-native';
import { QueryKey, useQuery, useQueryClient } from '@tanstack/react-query';
import { storageUtils } from '@services/storage';
import { useAppStore } from '@store/appStore';

const OFFLINE_CACHE_STORAGE_KEY = 'offline-cache:v1';
const CACHE_EXPIRY_MS = 24 * 60 * 60 * 1000; // 24 hours

export interface CacheEntry {
  key: string;
  data: unknown;
  timestamp: number;
  expiresAt: number;
  platform: string;
}

export interface OfflineCacheConfig {
  enabled?: boolean;
  maxSize?: number; // in bytes
  ttl?: number; // in milliseconds
  compress?: boolean;
}

export interface UseOfflineCacheResult {
  getCachedData: (key: QueryKey) => unknown | null;
  setCachedData: (key: QueryKey, data: unknown) => void;
  invalidateCache: (key?: QueryKey) => void;
  clearCache: () => void;
  getCacheSize: () => number;
  isCacheValid: (key: QueryKey) => boolean;
}

/**
 * Converts QueryKey to string for storage
 */
function serializeKey(key: QueryKey): string {
  return Array.isArray(key) ? key.join(':') : String(key);
}

/**
 * Reads all cache entries from storage
 */
function readCache(): Map<string, CacheEntry> {
  const cache = new Map<string, CacheEntry>();

  try {
    const value = storageUtils.getItem(OFFLINE_CACHE_STORAGE_KEY);
    if (!value) return cache;

    const entries: CacheEntry[] = JSON.parse(value);
    const now = Date.now();

    // Filter out expired entries
    for (const entry of entries) {
      if (entry.expiresAt > now) {
        cache.set(entry.key, entry);
      }
    }
  } catch (error) {
    console.warn('Failed to read offline cache:', error);
  }

  return cache;
}

/**
 * Writes cache entries to storage
 */
function writeCache(cache: Map<string, CacheEntry>): void {
  try {
    const entries = Array.from(cache.values());
    storageUtils.setItem(OFFLINE_CACHE_STORAGE_KEY, JSON.stringify(entries));
  } catch (error) {
    console.warn('Failed to write offline cache:', error);
  }
}

/**
 * Gets current cache size in bytes
 */
function getCacheSizeInBytes(cache: Map<string, CacheEntry>): number {
  let size = 0;
  for (const entry of cache.values()) {
    size += JSON.stringify(entry).length;
  }
  return size;
}

/**
 * Hook for managing offline data caching
 * Automatically caches successful queries when offline
 */
export function useOfflineCache(config?: OfflineCacheConfig): UseOfflineCacheResult {
  const [cache, setCache] = React.useState<Map<string, CacheEntry>>(() => readCache());
  const queryClient = useQueryClient();

  const ttl = config?.ttl || CACHE_EXPIRY_MS;
  const maxSize = config?.maxSize || 5 * 1024 * 1024; // 5 MB default
  const isEnabled = config?.enabled !== false;

  const getCachedData = React.useCallback(
    (key: QueryKey) => {
      if (!isEnabled) return null;

      const serializedKey = serializeKey(key);
      const entry = cache.get(serializedKey);

      if (!entry) return null;

      // Check if expired
      if (entry.expiresAt <= Date.now()) {
        const newCache = new Map(cache);
        newCache.delete(serializedKey);
        writeCache(newCache);
        setCache(newCache);
        return null;
      }

      return entry.data;
    },
    [cache, isEnabled]
  );

  const setCachedData = React.useCallback(
    (key: QueryKey, data: unknown) => {
      if (!isEnabled) return;

      const serializedKey = serializeKey(key);
      const now = Date.now();

      const entry: CacheEntry = {
        key: serializedKey,
        data,
        timestamp: now,
        expiresAt: now + ttl,
        platform: Platform.OS,
      };

      const newCache = new Map(cache);
      newCache.set(serializedKey, entry);

      // Check cache size and remove oldest entries if needed
      let cacheSize = getCacheSizeInBytes(newCache);
      if (cacheSize > maxSize) {
        const sortedEntries = Array.from(newCache.values()).sort(
          (a, b) => a.timestamp - b.timestamp
        );

        for (const oldEntry of sortedEntries) {
          if (cacheSize <= maxSize) break;
          cacheSize -= JSON.stringify(oldEntry).length;
          newCache.delete(oldEntry.key);
        }
      }

      writeCache(newCache);
      setCache(newCache);
    },
    [cache, isEnabled, ttl, maxSize]
  );

  const invalidateCache = React.useCallback(
    (key?: QueryKey) => {
      const newCache = new Map(cache);

      if (key) {
        const serializedKey = serializeKey(key);
        newCache.delete(serializedKey);
      } else {
        newCache.clear();
      }

      writeCache(newCache);
      setCache(newCache);
    },
    [cache]
  );

  const clearCache = React.useCallback(() => {
    storageUtils.removeItem(OFFLINE_CACHE_STORAGE_KEY);
    setCache(new Map());
  }, []);

  const getCacheSize = React.useCallback(() => {
    return getCacheSizeInBytes(cache);
  }, [cache]);

  const isCacheValid = React.useCallback(
    (key: QueryKey) => {
      const serializedKey = serializeKey(key);
      const entry = cache.get(serializedKey);
      return !!entry && entry.expiresAt > Date.now();
    },
    [cache]
  );

  // Auto-cache queries when online transitions to offline
  React.useEffect(() => {
    if (!isEnabled) return;

    const unsubscribe = useAppStore.subscribe(
      state => state.isOnline,
      (isOnlineNow, wasOnline) => {
        if (!isOnlineNow && wasOnline) {
          // Transitioning to offline - cache all current query data
          const queries = queryClient.getQueryCache().getAll();
          for (const query of queries) {
            if (query.state.data) {
              setCachedData(query.queryKey, query.state.data);
            }
          }
        }
      }
    );

    return unsubscribe;
  }, [isEnabled, queryClient, setCachedData]);

  return {
    getCachedData,
    setCachedData,
    invalidateCache,
    clearCache,
    getCacheSize,
    isCacheValid,
  };
}

/**
 * Custom hook that combines useQuery with offline caching
 * Falls back to cached data when offline
 */
export function useQueryWithOfflineCache<TData = unknown>(
  queryKey: QueryKey,
  queryFn: () => Promise<TData>,
  config?: OfflineCacheConfig
) {
  const { getCachedData, setCachedData, isCacheValid } = useOfflineCache(config);
  const { isOnline } = useAppStore();

  const query = useQuery({
    queryKey,
    queryFn: async () => {
      const data = await queryFn();
      // Cache successful response
      setCachedData(queryKey, data);
      return data;
    },
    staleTime: 5 * 60 * 1000, // 5 minutes
    enabled: isOnline,
  });

  // Fall back to cached data when offline
  const cachedData = React.useMemo(() => {
    if (isOnline || query.data) return null;
    return getCachedData(queryKey);
  }, [isOnline, query.data, queryKey, getCachedData]);

  return {
    ...query,
    data: query.data || (cachedData as TData | undefined),
    isUsingCache: !isOnline && !!cachedData && !query.data,
    isCacheValid: isCacheValid(queryKey),
  };
}
