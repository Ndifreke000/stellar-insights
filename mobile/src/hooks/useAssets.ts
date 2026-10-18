import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';

// ─── Types ───────────────────────────────────────────────────────────────────

export type AssetDataSource = 'live' | 'cache' | 'mock';

export interface AssetRecord {
  code: string;
  issuer: string;
  domain?: string;
  verified: boolean;
}

export interface AssetsListData {
  assets: AssetRecord[];
  total: number;
}

interface PaginatedAssetsResponse {
  data: AssetRecord[];
  pagination: {
    total: number;
  };
}

export interface UseAssetsReturn {
  assets: AssetRecord[];
  total: number;
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: AssetDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

// ─── Cache key ───────────────────────────────────────────────────────────────

export const ASSETS_LIST_CACHE_KEY = CACHE_KEYS.ASSETS;

// ─── Mock data ───────────────────────────────────────────────────────────────

const MOCK_ASSETS: AssetRecord[] = [
  {
    code: 'USDC',
    issuer: 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN',
    domain: 'centre.io',
    verified: true,
  },
  {
    code: 'EURC',
    issuer: 'GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y2IEMFDVXBSDP6SJY4ITNPP',
    domain: 'circle.com',
    verified: true,
  },
  {
    code: 'yXLM',
    issuer: 'GARDNV3Q7YGT4AKSDF25LT32YSCCW4EV22Y2TV3I2PU2MMXJTEDL5T55',
    domain: 'ultrastellar.com',
    verified: true,
  },
];

export function generateMockAssetsList(): AssetsListData {
  return {
    assets: MOCK_ASSETS,
    total: MOCK_ASSETS.length,
  };
}

// ─── Response normalisation ──────────────────────────────────────────────────

export function normalizeAssetsResponse(raw: unknown): AssetsListData {
  if (Array.isArray(raw)) {
    return { assets: raw as AssetRecord[], total: (raw as AssetRecord[]).length };
  }

  const response = raw as Partial<AssetsListData & PaginatedAssetsResponse>;

  if (Array.isArray(response.assets)) {
    return {
      assets: response.assets,
      total: response.total ?? response.assets.length,
    };
  }

  if (Array.isArray(response.data)) {
    return {
      assets: response.data,
      total: response.pagination?.total ?? response.data.length,
    };
  }

  throw new Error('Invalid assets response');
}

// ─── Cache helpers ───────────────────────────────────────────────────────────

async function readCachedAssets(): Promise<AssetsListData | null> {
  try {
    const cached = await AsyncStorage.getItem(ASSETS_LIST_CACHE_KEY);
    return cached ? (JSON.parse(cached) as AssetsListData) : null;
  } catch {
    return null;
  }
}

async function writeCachedAssets(data: AssetsListData): Promise<void> {
  try {
    await AsyncStorage.setItem(ASSETS_LIST_CACHE_KEY, JSON.stringify(data));
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

// ─── API fetch ───────────────────────────────────────────────────────────────

async function fetchAssetsList(): Promise<AssetsListData> {
  const response = await apiClient.get<unknown>('/assets');
  return normalizeAssetsResponse(response);
}

// ─── Fallback helper ─────────────────────────────────────────────────────────

function applyFallbackResult(
  setAssets: (assets: AssetRecord[]) => void,
  setTotal: (total: number) => void,
  setDataSource: (source: AssetDataSource) => void,
  setWarning: (warning: string) => void,
  dataSource: Extract<AssetDataSource, 'cache' | 'mock'>,
  warning: string,
  cachedData?: AssetsListData | null,
): void {
  const data =
    dataSource === 'cache' && cachedData ? cachedData : generateMockAssetsList();

  setAssets(data.assets);
  setTotal(data.total);
  setDataSource(dataSource);
  setWarning(warning);
}

// ─── Hook ────────────────────────────────────────────────────────────────────

/**
 * Fetches the list of Stellar assets with offline-first caching.
 *
 * Priority order:
 *   1. Live API (when online) → written to AsyncStorage cache
 *   2. AsyncStorage cache (when offline or API fails)
 *   3. In-memory mock data (labelled clearly)
 */
export function useAssets(): UseAssetsReturn {
  const [assets, setAssets] = useState<AssetRecord[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<AssetDataSource | null>(null);

  const loadAssets = useCallback(async () => {
    setLoading(true);
    setError(null);
    setWarning(null);
    setDataSource(null);

    let offline = false;
    try {
      const networkState = await NetInfo.fetch();
      offline = !(
        networkState.isConnected && networkState.isInternetReachable !== false
      );
    } catch {
      offline = true;
    }
    setIsOffline(offline);

    try {
      if (offline) {
        const cached = await readCachedAssets();
        if (cached) {
          setAssets(cached.assets);
          setTotal(cached.total);
          setDataSource('cache');
          setWarning('Offline — showing saved assets.');
          return;
        }

        applyFallbackResult(
          setAssets,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Offline — no saved data available. Showing sample assets.',
        );
        return;
      }

      try {
        const result = await fetchAssetsList();
        setAssets(result.assets);
        setTotal(result.total);
        setDataSource('live');
        await writeCachedAssets(result);
        return;
      } catch {
        const cached = await readCachedAssets();
        if (cached) {
          applyFallbackResult(
            setAssets,
            setTotal,
            setDataSource,
            setWarning,
            'cache',
            'Live data unavailable. Showing saved assets.',
            cached,
          );
          return;
        }

        applyFallbackResult(
          setAssets,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Live data unavailable. Showing sample assets.',
        );
      }
    } catch {
      setError('Failed to load assets');
      setAssets([]);
      setTotal(0);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadAssets();
  }, [loadAssets]);

  return {
    assets,
    total,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadAssets,
  };
}
