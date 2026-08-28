import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';
import {
  CorridorPerformance,
  DashboardData,
  DashboardDataSource,
  DashboardStats,
} from '@app-types/dashboard';

interface RawDashboardResponse {
  stats?: Partial<DashboardStats>;
  corridor_performance?: CorridorPerformance[];
  data?: DashboardData;
}

function isStats(value: unknown): value is DashboardStats {
  if (typeof value !== 'object' || value === null) {
    return false;
  }
  const stats = value as Record<string, unknown>;
  return (
    typeof stats.volume_24h === 'number' &&
    typeof stats.avg_success_rate === 'number' &&
    typeof stats.active_corridors === 'number'
  );
}

export function normalizeDashboardResponse(raw: unknown): DashboardData {
  const response = raw as RawDashboardResponse | undefined;

  // Some backends wrap the payload in a `data` envelope.
  const payload = response?.data ?? response;

  if (
    payload &&
    isStats(payload.stats) &&
    Array.isArray(payload.corridor_performance)
  ) {
    return {
      stats: payload.stats,
      corridor_performance: payload.corridor_performance,
    };
  }

  throw new Error('Invalid dashboard response');
}

export interface UseDashboardScreenReturn {
  stats: DashboardStats | null;
  corridorPerformance: CorridorPerformance[];
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: DashboardDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

export const DASHBOARD_CACHE_KEY = CACHE_KEYS.ANALYTICS;

const MOCK_STATS: DashboardStats = {
  volume_24h: 72000,
  volume_growth: 8.4,
  avg_success_rate: 96.1,
  success_rate_growth: 1.2,
  active_corridors: 28,
  corridors_growth: 3,
};

const MOCK_CORRIDOR_PERFORMANCE: CorridorPerformance[] = [
  { corridor: 'USDC→PHP', success_rate: 98.5, volume: 240000, health: 95 },
  { corridor: 'USD→PHP', success_rate: 97.2, volume: 180000, health: 92 },
  { corridor: 'EUR→USDC', success_rate: 99.1, volume: 150000, health: 98 },
  { corridor: 'USDC→SGD', success_rate: 96.8, volume: 120000, health: 89 },
  { corridor: 'USD→EUR', success_rate: 98.9, volume: 200000, health: 97 },
];

export function generateMockDashboard(): DashboardData {
  return {
    stats: MOCK_STATS,
    corridor_performance: MOCK_CORRIDOR_PERFORMANCE,
  };
}

async function readCachedDashboard(): Promise<DashboardData | null> {
  try {
    const cached = await AsyncStorage.getItem(DASHBOARD_CACHE_KEY);
    return cached ? (JSON.parse(cached) as DashboardData) : null;
  } catch {
    return null;
  }
}

async function writeCachedDashboard(data: DashboardData): Promise<void> {
  try {
    await AsyncStorage.setItem(DASHBOARD_CACHE_KEY, JSON.stringify(data));
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

async function fetchDashboard(): Promise<DashboardData> {
  const response = await apiClient.get<unknown>('/analytics/dashboard');
  return normalizeDashboardResponse(response);
}

export function useDashboardScreen(): UseDashboardScreenReturn {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [corridorPerformance, setCorridorPerformance] = useState<
    CorridorPerformance[]
  >([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<DashboardDataSource | null>(null);

  const applyData = useCallback((data: DashboardData) => {
    setStats(data.stats);
    setCorridorPerformance(data.corridor_performance);
  }, []);

  const loadDashboard = useCallback(async () => {
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
        const cached = await readCachedDashboard();
        if (cached) {
          applyData(cached);
          setDataSource('cache');
          setWarning('Offline — showing saved dashboard.');
          return;
        }

        applyData(generateMockDashboard());
        setDataSource('mock');
        setWarning(
          'Offline — no saved data available. Showing sample dashboard.',
        );
        return;
      }

      try {
        const result = await fetchDashboard();
        applyData(result);
        setDataSource('live');
        await writeCachedDashboard(result);
        return;
      } catch {
        const cached = await readCachedDashboard();
        if (cached) {
          applyData(cached);
          setDataSource('cache');
          setWarning('Live data unavailable. Showing saved dashboard.');
          return;
        }

        applyData(generateMockDashboard());
        setDataSource('mock');
        setWarning('Live data unavailable. Showing sample dashboard.');
      }
    } catch {
      setError('Failed to load dashboard');
      setStats(null);
      setCorridorPerformance([]);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, [applyData]);

  useEffect(() => {
    void loadDashboard();
  }, [loadDashboard]);

  return {
    stats,
    corridorPerformance,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadDashboard,
  };
}
