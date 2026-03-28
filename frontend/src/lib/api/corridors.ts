import { api} from "./api";
import {LatencyDataPoint, LiquidityDataPoint, SlippageDataPoint, SuccessRateDataPoint, VolumeDataPoint } from "./types";


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
  liquidity_trend: "increasing" | "stable" | "decreasing";
  average_slippage_bps: number;
  health_score: number;
  last_updated: string;
}

export interface CorridorDetailData {
  corridor: CorridorMetrics;
  historical_success_rate: SuccessRateDataPoint[];
  latency_distribution: LatencyDataPoint[];
  liquidity_trends: LiquidityDataPoint[];
  historical_volume: VolumeDataPoint[];
  historical_slippage: SlippageDataPoint[];
  related_corridors?: CorridorMetrics[];
}

/**
 * Fetch corridor metrics by ID
 */
export async function getCorridorDetail(
  corridorId: string,
): Promise<CorridorDetailData> {
  return api.get<CorridorDetailData>(`/corridors/${corridorId}`);
}

/**
 * Fetch all corridors (for listing and navigation)
 */
export interface CorridorFilters {
  success_rate_min?: number;
  success_rate_max?: number;
  volume_min?: number;
  volume_max?: number;
  asset_code?: string;
  time_period?: "7d" | "30d" | "90d" | "";
  limit?: number;
  offset?: number;
  sort_by?: "success_rate" | "health_score" | "liquidity";
}

export async function getCorridors(
  filters?: CorridorFilters,
): Promise<CorridorMetrics[]> {
  const params = new URLSearchParams();
  if (filters) {
    if (filters.success_rate_min !== undefined)
      params.append("success_rate_min", filters.success_rate_min.toString());
    if (filters.success_rate_max !== undefined)
      params.append("success_rate_max", filters.success_rate_max.toString());
    if (filters.volume_min !== undefined)
      params.append("volume_min", filters.volume_min.toString());
    if (filters.volume_max !== undefined)
      params.append("volume_max", filters.volume_max.toString());
    if (filters.asset_code) params.append("asset_code", filters.asset_code);
    if (filters.time_period) params.append("time_period", filters.time_period);
    if (filters.limit !== undefined)
      params.append("limit", filters.limit.toString());
    if (filters.offset !== undefined)
      params.append("offset", filters.offset.toString());
    if (filters.sort_by) params.append("sort_by", filters.sort_by);
  }
  const query = params.toString();
  const url = query ? `/corridors?${query}` : "/corridors";
  return api.get<CorridorMetrics[]>(url);
}

/**
 * Mock data generator for development (fallback if API not available)
 */
export function generateMockCorridorData(
  corridorId: string,
): CorridorDetailData {
  const now = new Date();
  const monthAgo = new Date(now.getTime() - 30 * 24 * 60 * 60 * 1000);

  // Generate historical success rate data
  const historical_success_rate: SuccessRateDataPoint[] = [];
  for (let i = 0; i < 30; i++) {
    const date = new Date(monthAgo.getTime() + i * 24 * 60 * 60 * 1000);
    historical_success_rate.push({
      timestamp: date.toISOString().split("T")[0],
      success_rate: 85 + Math.random() * 10 - 5,
      attempts: Math.floor(100 + Math.random() * 200),
    });
  }

  // Generate latency distribution
  const latency_distribution: LatencyDataPoint[] = [
    { latency_bucket_ms: 100, count: 250, percentage: 15 },
    { latency_bucket_ms: 250, count: 520, percentage: 31 },
    { latency_bucket_ms: 500, count: 580, percentage: 35 },
    { latency_bucket_ms: 1000, count: 280, percentage: 17 },
    { latency_bucket_ms: 2000, count: 50, percentage: 3 },
  ];

  // Generate volume and slippage history
  const historical_volume: VolumeDataPoint[] = [];
  const historical_slippage: SlippageDataPoint[] = [];

  for (let i = 0; i < 30; i++) {
    const date = new Date(monthAgo.getTime() + i * 24 * 60 * 60 * 1000);
    const dateStr = date.toISOString().split("T")[0];

    historical_volume.push({
      timestamp: dateStr,
      volume_usd: 800000 + Math.random() * 400000 - 200000,
    });

    historical_slippage.push({
      timestamp: dateStr,
      average_slippage_bps: 15 + Math.random() * 10 - 5,
    });
  }

  // Generate liquidity trends
  const liquidity_trends: LiquidityDataPoint[] = [];
  for (let i = 0; i < 30; i++) {
    const date = new Date(monthAgo.getTime() + i * 24 * 60 * 60 * 1000);
    liquidity_trends.push({
      timestamp: date.toISOString().split("T")[0],
      liquidity_usd: 5000000 + Math.random() * 2000000 - 1000000,
      volume_24h_usd: 500000 + Math.random() * 300000 - 150000,
    });
  }

  return {
    corridor: {
      id: corridorId,
      source_asset: "USDC",
      destination_asset: "PHP",
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
      liquidity_trend: "increasing",
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
        id: "corridor-2",
        source_asset: "USDC",
        destination_asset: "JPY",
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
        liquidity_trend: "stable",
        average_slippage_bps: 18.2,
        health_score: 85,
        last_updated: new Date().toISOString(),
      },
    ],
  };
}