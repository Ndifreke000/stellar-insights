const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001/api';

export interface CorridorMetrics {
  id: string;
  source_asset: string;
  destination_asset: string;
  success_rate: number;
  total_attempts: number;
  successful_payments: number;
  failed_payments: number;
  average_latency_ms: number;
  median_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
  liquidity_depth_usd: number;
  liquidity_volume_24h_usd: number;
  liquidity_trend: 'increasing' | 'stable' | 'decreasing';
  health_score: number;
  last_updated: string;
}

export interface SuccessRateDataPoint {
  timestamp: string;
  success_rate: number;
  attempts: number;
}

export interface LatencyDataPoint {
  latency_bucket_ms: number;
  count: number;
  percentage: number;
}

export interface LiquidityDataPoint {
  timestamp: string;
  liquidity_usd: number;
  volume_24h_usd: number;
}

export interface CorridorDetailData {
  corridor: CorridorMetrics;
  historical_success_rate: SuccessRateDataPoint[];
  latency_distribution: LatencyDataPoint[];
  liquidity_trends: LiquidityDataPoint[];
  related_corridors?: CorridorMetrics[];
}

export async function getCorridorDetail(corridorId: string): Promise<CorridorDetailData> {
  const response = await fetch(`${API_BASE_URL}/corridors/${corridorId}`, {
    method: 'GET',
    headers: { 'Content-Type': 'application/json' },
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }

  return await response.json();
}

export async function getCorridors(): Promise<CorridorMetrics[]> {
  const response = await fetch(`${API_BASE_URL}/corridors`, {
    method: 'GET',
    headers: { 'Content-Type': 'application/json' },
  });

  if (!response.ok) {
    throw new Error(`API error: ${response.status}`);
  }

  return await response.json();
}

export function generateMockCorridorData(corridorId: string): CorridorDetailData {
  const now = new Date();
  const monthAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);

  const historical_success_rate: SuccessRateDataPoint[] = [];
  for (let i = 0; i < 30; i++) {
    const date = new Date(monthAgo.getTime() + i * 24 * 60 * 60 * 1000);
    historical_success_rate.push({
      timestamp: date.toISOString().split('T')[0]!,
      success_rate: 85 + Math.random() * 10 - 5,
      attempts: Math.floor(100 + Math.random() * 200),
    });
  }

  const latency_distribution: LatencyDataPoint[] = [
    { latency_bucket_ms: 100, count: 250, percentage: 15 },
    { latency_bucket_ms: 250, count: 520, percentage: 31 },
    { latency_bucket_ms: 500, count: 580, percentage: 35 },
    { latency_bucket_ms: 1000, count: 280, percentage: 17 },
    { latency_bucket_ms: 2000, count: 50, percentage: 3 },
  ];

  const liquidity_trends: LiquidityDataPoint[] = [];
  for (let i = 0; i < 30; i++) {
    const date = new Date(monthAgo.getTime() + i * 24 * 60 * 60 * 1000);
    liquidity_trends.push({
      timestamp: date.toISOString().split('T')[0]!,
      liquidity_usd: 5000000 + Math.random() * 2000000 - 1000000,
      volume_24h_usd: 500000 + Math.random() * 300000 - 150000,
    });
  }

  return {
    corridor: {
      id: corridorId,
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
      health_score: 94,
      last_updated: new Date().toISOString(),
    },
    historical_success_rate,
    latency_distribution,
    liquidity_trends,
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
        health_score: 85,
        last_updated: new Date().toISOString(),
      },
    ],
  };
}
