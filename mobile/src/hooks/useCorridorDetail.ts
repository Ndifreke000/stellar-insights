import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import {
  CORRIDOR_DETAIL_CACHE_PREFIX,
  CorridorDataSource,
  CorridorDetailData,
  UseCorridorDetailOptions,
  UseCorridorDetailReturn,
} from '@app-types/corridor';

export function parseCorridorAssets(corridorId: string): [string, string] {
  if (corridorId.includes('->')) {
    const [sourcePart, destinationPart] = corridorId.split('->');
    return [
      sourcePart.split(':')[0] || 'USDC',
      destinationPart.split(':')[0] || 'PHP',
    ];
  }

  const separatorIndex = corridorId.indexOf('-');
  if (separatorIndex === -1) {
    return ['USDC', 'PHP'];
  }

  return [
    corridorId.slice(0, separatorIndex),
    corridorId.slice(separatorIndex + 1),
  ];
}

export function generateMockCorridorData(corridorId: string): CorridorDetailData {
  const now = new Date();
  const monthAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);
  const [sourceAsset, destinationAsset] = parseCorridorAssets(corridorId);

  const historical_success_rate = Array.from({ length: 30 }, (_, index) => {
    const date = new Date(monthAgo.getTime() + index * 24 * 60 * 60 * 1000);
    return {
      timestamp: date.toISOString().split('T')[0],
      success_rate: 85 + Math.random() * 10 - 5,
      attempts: Math.floor(100 + Math.random() * 200),
    };
  });

  const latency_distribution = [
    { latency_bucket_ms: 100, count: 250, percentage: 15 },
    { latency_bucket_ms: 250, count: 520, percentage: 31 },
    { latency_bucket_ms: 500, count: 580, percentage: 35 },
    { latency_bucket_ms: 1000, count: 280, percentage: 17 },
    { latency_bucket_ms: 2000, count: 50, percentage: 3 },
  ];

  const historical_volume = Array.from({ length: 30 }, (_, index) => {
    const date = new Date(monthAgo.getTime() + index * 24 * 60 * 60 * 1000);
    return {
      timestamp: date.toISOString().split('T')[0],
      volume_usd: 800000 + Math.random() * 400000 - 200000,
    };
  });

  const historical_slippage = Array.from({ length: 30 }, (_, index) => {
    const date = new Date(monthAgo.getTime() + index * 24 * 60 * 60 * 1000);
    return {
      timestamp: date.toISOString().split('T')[0],
      average_slippage_bps: 15 + Math.random() * 10 - 5,
    };
  });

  const liquidity_trends = Array.from({ length: 30 }, (_, index) => {
    const date = new Date(monthAgo.getTime() + index * 24 * 60 * 60 * 1000);
    return {
      timestamp: date.toISOString().split('T')[0],
      liquidity_usd: 5000000 + Math.random() * 2000000 - 1000000,
      volume_24h_usd: 500000 + Math.random() * 300000 - 150000,
    };
  });

  return {
    corridor: {
      id: corridorId,
      source_asset: sourceAsset,
      destination_asset: destinationAsset,
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
    historical_success_rate,
    latency_distribution,
    liquidity_trends,
    historical_volume,
    historical_slippage,
    related_corridors: [
      {
        id: 'corridor-2',
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
    ],
  };
}

async function readCachedCorridor(
  corridorId: string,
): Promise<CorridorDetailData | null> {
  try {
    const cached = await AsyncStorage.getItem(
      `${CORRIDOR_DETAIL_CACHE_PREFIX}${corridorId}`,
    );
    return cached ? (JSON.parse(cached) as CorridorDetailData) : null;
  } catch {
    return null;
  }
}

async function writeCachedCorridor(
  corridorId: string,
  data: CorridorDetailData,
): Promise<void> {
  try {
    await AsyncStorage.setItem(
      `${CORRIDOR_DETAIL_CACHE_PREFIX}${corridorId}`,
      JSON.stringify(data),
    );
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

async function fetchCorridorDetail(corridorId: string): Promise<CorridorDetailData> {
  return apiClient.get<CorridorDetailData>(
    `/corridors/${encodeURIComponent(corridorId)}`,
  );
}

function applyFallbackResult(
  setData: (data: CorridorDetailData) => void,
  setDataSource: (source: CorridorDataSource) => void,
  setWarning: (warning: string) => void,
  corridorId: string,
  dataSource: Extract<CorridorDataSource, 'cache' | 'mock'>,
  warning: string,
  cachedData?: CorridorDetailData | null,
): void {
  const data =
    dataSource === 'cache' && cachedData
      ? cachedData
      : generateMockCorridorData(corridorId);

  setData(data);
  setDataSource(dataSource);
  setWarning(warning);
}

export function useCorridorDetail({
  corridorId,
}: UseCorridorDetailOptions): UseCorridorDetailReturn {
  const [data, setData] = useState<CorridorDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<CorridorDataSource | null>(null);

  const loadCorridorDetail = useCallback(async () => {
    if (!corridorId) {
      setError('Corridor ID is required');
      setWarning(null);
      setData(null);
      setDataSource(null);
      setLoading(false);
      return;
    }

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
        const cached = await readCachedCorridor(corridorId);
        if (cached) {
          setData(cached);
          setDataSource('cache');
          setWarning('Offline — showing saved corridor data.');
          return;
        }

        applyFallbackResult(
          setData,
          setDataSource,
          setWarning,
          corridorId,
          'mock',
          'Offline — no saved data available. Showing sample data.',
        );
        return;
      }

      try {
        const result = await fetchCorridorDetail(corridorId);
        setData(result);
        setDataSource('live');
        await writeCachedCorridor(corridorId, result);
        return;
      } catch {
        const cached = await readCachedCorridor(corridorId);
        if (cached) {
          applyFallbackResult(
            setData,
            setDataSource,
            setWarning,
            corridorId,
            'cache',
            'Live data unavailable. Showing saved corridor data.',
            cached,
          );
          return;
        }

        applyFallbackResult(
          setData,
          setDataSource,
          setWarning,
          corridorId,
          'mock',
          'Live data unavailable. Showing sample data.',
        );
      }
    } catch {
      setError('Failed to load corridor data');
      setData(null);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, [corridorId]);

  useEffect(() => {
    void loadCorridorDetail();
  }, [loadCorridorDetail]);

  return {
    data,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadCorridorDetail,
  };
}
