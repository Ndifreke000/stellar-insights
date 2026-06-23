import { useCallback } from 'react';

import { apiClient } from '@services/api';
import { CACHE_KEYS } from '@config/constants';
import { CorridorMetrics } from '@types/corridor';
import {
  CursorPage,
  usePaginatedList,
  UsePaginatedListReturn,
} from './usePaginatedList';

interface CorridorsCursorResponse {
  data: CorridorMetrics[];
  next_cursor: string | null;
  total: number;
}

function normalizeCorridorsPage(raw: unknown): CursorPage<CorridorMetrics> {
  // Cursor-based envelope: { data, next_cursor, total }
  const r = raw as Partial<CorridorsCursorResponse>;
  if (Array.isArray(r.data)) {
    return {
      items: r.data,
      next_cursor: r.next_cursor ?? null,
      total: r.total ?? r.data.length,
    };
  }

  // Legacy: plain array (no pagination)
  if (Array.isArray(raw)) {
    return { items: raw as CorridorMetrics[], next_cursor: null, total: (raw as CorridorMetrics[]).length };
  }

  throw new Error('Invalid corridors response');
}

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

function mockCorridorsPage(): CursorPage<CorridorMetrics> {
  return { items: MOCK_CORRIDORS, next_cursor: null, total: MOCK_CORRIDORS.length };
}

export type UseCorridorsListReturn = UsePaginatedListReturn<CorridorMetrics>;

export function useCorridorsList(): UseCorridorsListReturn {
  const fetchPage = useCallback(async (cursor: string | null) => {
    const params = cursor ? { cursor, limit: 20 } : { limit: 20 };
    const response = await apiClient.get<unknown>('/corridors', { params });
    return normalizeCorridorsPage(response);
  }, []);

  return usePaginatedList<CorridorMetrics>({
    cacheKey: CACHE_KEYS.CORRIDORS,
    fetchPage,
    mockData: mockCorridorsPage,
  });
}
