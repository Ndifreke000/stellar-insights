import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';
import { CorridorDataSource, CorridorMetrics } from '@types/corridor';

export interface CorridorsListData {
  corridors: CorridorMetrics[];
  total: number;
}

interface PaginatedCorridorsResponse {
  data: CorridorMetrics[];
  pagination: {
    total: number;
  };
}

export function normalizeCorridorsResponse(raw: unknown): CorridorsListData {
  if (Array.isArray(raw)) {
    return { corridors: raw, total: raw.length };
  }

  const response = raw as Partial<CorridorsListData & PaginatedCorridorsResponse>;

  if (Array.isArray(response.corridors)) {
    return {
      corridors: response.corridors,
      total: response.total ?? response.corridors.length,
    };
  }

  if (Array.isArray(response.data)) {
    return {
      corridors: response.data,
      total: response.pagination?.total ?? response.data.length,
    };
  }

  throw new Error('Invalid corridors response');
}

export interface UseCorridorsListReturn {
  corridors: CorridorMetrics[];
  total: number;
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: CorridorDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

export const CORRIDORS_LIST_CACHE_KEY = CACHE_KEYS.CORRIDORS;

const MOCK_CORRIDORS: CorridorMetrics[] = [
  {
    id: 'USDC-PHP',
    source_asset: 'USDC',
    destination_asset: 'PHP',
    success_rate: 92.5,
    total_attempts: 1678,
    successful_payments: 1552,
    failed_payments: 126,
    average_latency_ms: 487,
    median_latency_ms: 350,
    p95_latency_ms: 1250,
    p99_latency_ms: 1950,
    liquidity_depth_usd: 6200000,
    liquidity_volume_24h_usd: 850000,
    liquidity_trend: 'increasing',
    average_slippage_bps: 12.5,
    health_score: 94,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'USDC-JPY',
    source_asset: 'USDC',
    destination_asset: 'JPY',
    success_rate: 88.3,
    total_attempts: 1200,
    successful_payments: 1060,
    failed_payments: 140,
    average_latency_ms: 520,
    median_latency_ms: 380,
    p95_latency_ms: 1400,
    p99_latency_ms: 2100,
    liquidity_depth_usd: 4500000,
    liquidity_volume_24h_usd: 620000,
    liquidity_trend: 'stable',
    average_slippage_bps: 18.2,
    health_score: 85,
    last_updated: new Date().toISOString(),
  },
  {
    id: 'EURC-NGN',
    source_asset: 'EURC',
    destination_asset: 'NGN',
    success_rate: 76.4,
    total_attempts: 890,
    successful_payments: 680,
    failed_payments: 210,
    average_latency_ms: 720,
    median_latency_ms: 540,
    p95_latency_ms: 1800,
    p99_latency_ms: 2600,
    liquidity_depth_usd: 2100000,
    liquidity_volume_24h_usd: 310000,
    liquidity_trend: 'decreasing',
    average_slippage_bps: 24.8,
    health_score: 68,
    last_updated: new Date().toISOString(),
  },
];

export function generateMockCorridorsList(): CorridorsListData {
  return {
    corridors: MOCK_CORRIDORS,
    total: MOCK_CORRIDORS.length,
  };
}

async function readCachedCorridors(): Promise<CorridorsListData | null> {
  try {
    const cached = await AsyncStorage.getItem(CORRIDORS_LIST_CACHE_KEY);
    return cached ? (JSON.parse(cached) as CorridorsListData) : null;
  } catch {
    return null;
  }
}

async function writeCachedCorridors(data: CorridorsListData): Promise<void> {
  try {
    await AsyncStorage.setItem(CORRIDORS_LIST_CACHE_KEY, JSON.stringify(data));
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

async function fetchCorridorsList(): Promise<CorridorsListData> {
  const response = await apiClient.get<unknown>('/corridors');
  return normalizeCorridorsResponse(response);
}

function applyFallbackResult(
  setCorridors: (corridors: CorridorMetrics[]) => void,
  setTotal: (total: number) => void,
  setDataSource: (source: CorridorDataSource) => void,
  setWarning: (warning: string) => void,
  dataSource: Extract<CorridorDataSource, 'cache' | 'mock'>,
  warning: string,
  cachedData?: CorridorsListData | null,
): void {
  const data =
    dataSource === 'cache' && cachedData
      ? cachedData
      : generateMockCorridorsList();

  setCorridors(data.corridors);
  setTotal(data.total);
  setDataSource(dataSource);
  setWarning(warning);
}

export function useCorridorsList(): UseCorridorsListReturn {
  const [corridors, setCorridors] = useState<CorridorMetrics[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<CorridorDataSource | null>(null);

  const loadCorridorsList = useCallback(async () => {
    setLoading(true);
    setError(null);
    setWarning(null);
    setDataSource(null);

    let offline = false;
    try {
      const networkState = await NetInfo.fetch();
      offline = !(
        networkState.isConnected &&
        networkState.isInternetReachable !== false
      );
    } catch {
      offline = true;
    }
    setIsOffline(offline);

    try {
      if (offline) {
        const cached = await readCachedCorridors();
        if (cached) {
          setCorridors(cached.corridors);
          setTotal(cached.total);
          setDataSource('cache');
          setWarning('Offline — showing saved corridors.');
          return;
        }

        applyFallbackResult(
          setCorridors,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Offline — no saved data available. Showing sample corridors.',
        );
        return;
      }

      try {
        const result = await fetchCorridorsList();
        setCorridors(result.corridors);
        setTotal(result.total);
        setDataSource('live');
        await writeCachedCorridors(result);
        return;
      } catch {
        const cached = await readCachedCorridors();
        if (cached) {
          applyFallbackResult(
            setCorridors,
            setTotal,
            setDataSource,
            setWarning,
            'cache',
            'Live data unavailable. Showing saved corridors.',
            cached,
          );
          return;
        }

        applyFallbackResult(
          setCorridors,
          setTotal,
          setDataSource,
          setWarning,
          'mock',
          'Live data unavailable. Showing sample corridors.',
        );
      }
    } catch {
      setError('Failed to load corridors');
      setCorridors([]);
      setTotal(0);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void loadCorridorsList();
  }, [loadCorridorsList]);

  return {
    corridors,
    total,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadCorridorsList,
  };
}
