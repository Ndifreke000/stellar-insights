import AsyncStorage from '@react-native-async-storage/async-storage';
import NetInfo from '@react-native-community/netinfo';
import { useCallback, useEffect, useState } from 'react';

import { apiClient } from '@services/api';
import {
  ANCHOR_DETAIL_CACHE_PREFIX,
  AnchorDataSource,
  AnchorDetailData,
  AnchorMetrics,
  IssuedAsset,
  ReliabilityDataPoint,
  UseAnchorDetailOptions,
  UseAnchorDetailReturn,
} from '@app-types/anchor';

function isStellarAccountAddress(value: string): boolean {
  const trimmed = value.trim();
  return (
    trimmed.length === 56 && (trimmed.startsWith('G') || trimmed.startsWith('M'))
  );
}

function computeFailureRate(anchor: {
  total_transactions: number;
  failed_transactions: number;
  failure_rate?: number;
}): number {
  if (typeof anchor.failure_rate === 'number') {
    return anchor.failure_rate;
  }
  if (anchor.total_transactions <= 0) {
    return 0;
  }
  return (anchor.failed_transactions / anchor.total_transactions) * 100;
}

function normalizeIssuedAssets(raw: unknown, stellarAccount: string): IssuedAsset[] {
  const assets = raw as
    | { assets?: unknown[]; issued_assets?: IssuedAsset[] }
    | unknown[];

  if (Array.isArray((assets as { issued_assets?: IssuedAsset[] }).issued_assets)) {
    return (assets as { issued_assets: IssuedAsset[] }).issued_assets;
  }

  const list = Array.isArray(assets)
    ? assets
    : Array.isArray((assets as { assets?: unknown[] }).assets)
      ? (assets as { assets: unknown[] }).assets
      : [];

  return list.map(item => {
    const asset = item as Record<string, unknown>;
    const totalTx = Number(asset.total_transactions ?? 0);
    const successRate = Number(asset.success_rate ?? 100);
    return {
      asset_code: String(asset.asset_code ?? ''),
      issuer: String(asset.issuer ?? asset.asset_issuer ?? stellarAccount),
      volume_24h_usd: Number(asset.volume_24h_usd ?? asset.volume_usd ?? 0),
      success_rate: successRate,
      failure_rate: Number(asset.failure_rate ?? Math.max(0, 100 - successRate)),
      total_transactions: totalTx,
    };
  });
}

function normalizeReliabilityHistory(raw: unknown): ReliabilityDataPoint[] {
  const response = raw as {
    reliability_history?: ReliabilityDataPoint[];
    metrics_history?: Array<{
      timestamp: string;
      reliability_score: number;
      score?: number;
    }>;
  };

  if (Array.isArray(response.reliability_history)) {
    return response.reliability_history;
  }

  if (!Array.isArray(response.metrics_history)) {
    return [];
  }

  return response.metrics_history.map(point => ({
    timestamp:
      typeof point.timestamp === 'string'
        ? point.timestamp.split('T')[0]
        : String(point.timestamp),
    score: point.score ?? point.reliability_score,
  }));
}

export function normalizeAnchorDetailResponse(
  raw: unknown,
  fallbackId: string,
): AnchorDetailData {
  const response = raw as Record<string, unknown>;
  const anchorRaw = (response.anchor ?? {}) as Record<string, unknown>;
  const stellarAccount = String(
    anchorRaw.stellar_account ?? anchorRaw.stellarAccount ?? fallbackId,
  );
  const issuedAssets = normalizeIssuedAssets(response, stellarAccount);
  const totalTransactions = Number(
    anchorRaw.total_transactions ?? anchorRaw.totalTransactions ?? 0,
  );
  const failedTransactions = Number(
    anchorRaw.failed_transactions ?? anchorRaw.failedTransactions ?? 0,
  );
  const successfulTransactions = Number(
    anchorRaw.successful_transactions ??
      anchorRaw.successfulTransactions ??
      Math.max(0, totalTransactions - failedTransactions),
  );

  const anchor: AnchorMetrics = {
    id: String(anchorRaw.id ?? fallbackId),
    name: String(anchorRaw.name ?? 'Unknown Anchor'),
    stellar_account: stellarAccount,
    reliability_score: Number(anchorRaw.reliability_score ?? anchorRaw.reliabilityScore ?? 0),
    asset_coverage: Number(
      anchorRaw.asset_coverage ?? anchorRaw.assetCoverage ?? issuedAssets.length,
    ),
    failure_rate: computeFailureRate({
      total_transactions: totalTransactions,
      failed_transactions: failedTransactions,
      failure_rate: anchorRaw.failure_rate as number | undefined,
    }),
    total_transactions: totalTransactions,
    successful_transactions: successfulTransactions,
    failed_transactions: failedTransactions,
    status: String(anchorRaw.status ?? 'unknown'),
  };

  return {
    anchor,
    issued_assets: issuedAssets,
    reliability_history: normalizeReliabilityHistory(response),
    top_failure_reasons: response.top_failure_reasons as AnchorDetailData['top_failure_reasons'],
    recent_failed_corridors: response.recent_failed_corridors as AnchorDetailData['recent_failed_corridors'],
  };
}

export function generateMockAnchorDetail(anchorId: string): AnchorDetailData {
  const now = new Date();
  const reliability_history: ReliabilityDataPoint[] = [];

  for (let i = 29; i >= 0; i--) {
    const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
    reliability_history.push({
      timestamp: date.toISOString().split('T')[0],
      score: 85 + Math.random() * 15 - (Math.random() > 0.8 ? 10 : 0),
    });
  }

  const stellarAccount = isStellarAccountAddress(anchorId)
    ? anchorId
    : 'GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN';

  const issued_assets: IssuedAsset[] = [
    {
      asset_code: 'USDC',
      issuer: stellarAccount,
      volume_24h_usd: 1_250_000,
      success_rate: 98.5,
      failure_rate: 1.5,
      total_transactions: 5400,
    },
    {
      asset_code: 'EURC',
      issuer: stellarAccount,
      volume_24h_usd: 450_000,
      success_rate: 94.2,
      failure_rate: 5.8,
      total_transactions: 1200,
    },
  ];

  return {
    anchor: {
      id: anchorId,
      name: 'Simulated Anchor Inc.',
      stellar_account: stellarAccount,
      reliability_score: reliability_history[reliability_history.length - 1].score,
      asset_coverage: issued_assets.length,
      failure_rate: 2.1,
      total_transactions: 6600,
      successful_transactions: 6461,
      failed_transactions: 139,
      status: 'green',
    },
    issued_assets,
    reliability_history,
    top_failure_reasons: [
      { reason: 'Timeout awaiting response', count: 45 },
      { reason: 'Insufficient liquidity', count: 23 },
      { reason: 'Path payment failed', count: 12 },
    ],
    recent_failed_corridors: [
      {
        corridor_id: 'USDC-PHP',
        timestamp: new Date(now.getTime() - 1000 * 60 * 15).toISOString(),
      },
      {
        corridor_id: 'EURC-NGN',
        timestamp: new Date(now.getTime() - 1000 * 60 * 145).toISOString(),
      },
    ],
  };
}

async function readCachedAnchor(anchorId: string): Promise<AnchorDetailData | null> {
  try {
    const cached = await AsyncStorage.getItem(
      `${ANCHOR_DETAIL_CACHE_PREFIX}${anchorId}`,
    );
    return cached ? (JSON.parse(cached) as AnchorDetailData) : null;
  } catch {
    return null;
  }
}

async function writeCachedAnchor(anchorId: string, data: AnchorDetailData): Promise<void> {
  try {
    await AsyncStorage.setItem(
      `${ANCHOR_DETAIL_CACHE_PREFIX}${anchorId}`,
      JSON.stringify(data),
    );
  } catch {
    // Cache writes are best-effort for offline support.
  }
}

async function fetchAnchorDetail(anchorId: string): Promise<AnchorDetailData> {
  const trimmed = anchorId.trim();
  const path = isStellarAccountAddress(trimmed)
    ? `/anchors/account/${encodeURIComponent(trimmed)}`
    : `/anchors/${encodeURIComponent(trimmed)}`;

  const response = await apiClient.get<unknown>(path);
  return normalizeAnchorDetailResponse(response, trimmed);
}

function applyFallbackResult(
  setData: (data: AnchorDetailData) => void,
  setDataSource: (source: AnchorDataSource) => void,
  setWarning: (warning: string) => void,
  anchorId: string,
  dataSource: Extract<AnchorDataSource, 'cache' | 'mock'>,
  warning: string,
  cachedData?: AnchorDetailData | null,
): void {
  const data =
    dataSource === 'cache' && cachedData
      ? cachedData
      : generateMockAnchorDetail(anchorId);

  setData(data);
  setDataSource(dataSource);
  setWarning(warning);
}

export function useAnchorDetail({
  anchorId,
}: UseAnchorDetailOptions): UseAnchorDetailReturn {
  const [data, setData] = useState<AnchorDetailData | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [warning, setWarning] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);
  const [dataSource, setDataSource] = useState<AnchorDataSource | null>(null);

  const loadAnchorDetail = useCallback(async () => {
    if (!anchorId) {
      setError('Anchor ID is required');
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
        const cached = await readCachedAnchor(anchorId);
        if (cached) {
          setData(cached);
          setDataSource('cache');
          setWarning('Offline — showing saved anchor data.');
          return;
        }

        applyFallbackResult(
          setData,
          setDataSource,
          setWarning,
          anchorId,
          'mock',
          'Offline — no saved data available. Showing sample data.',
        );
        return;
      }

      try {
        const result = await fetchAnchorDetail(anchorId);
        setData(result);
        setDataSource('live');
        await writeCachedAnchor(anchorId, result);
        return;
      } catch {
        const cached = await readCachedAnchor(anchorId);
        if (cached) {
          applyFallbackResult(
            setData,
            setDataSource,
            setWarning,
            anchorId,
            'cache',
            'Live data unavailable. Showing saved anchor data.',
            cached,
          );
          return;
        }

        applyFallbackResult(
          setData,
          setDataSource,
          setWarning,
          anchorId,
          'mock',
          'Live data unavailable. Showing sample data.',
        );
      }
    } catch {
      setError('Failed to load anchor data');
      setData(null);
      setDataSource(null);
      setWarning(null);
    } finally {
      setLoading(false);
    }
  }, [anchorId]);

  useEffect(() => {
    void loadAnchorDetail();
  }, [loadAnchorDetail]);

  return {
    data,
    loading,
    error,
    warning,
    isOffline,
    dataSource,
    isFromCache: dataSource === 'cache',
    refetch: loadAnchorDetail,
  };
}
