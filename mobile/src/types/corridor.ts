export type LiquidityTrend = 'increasing' | 'stable' | 'decreasing';

export type CorridorDataSource = 'live' | 'cache' | 'mock';

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
  liquidity_trend: LiquidityTrend;
  average_slippage_bps: number;
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

export interface VolumeDataPoint {
  timestamp: string;
  volume_usd: number;
}

export interface SlippageDataPoint {
  timestamp: string;
  average_slippage_bps: number;
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

export interface UseCorridorDetailOptions {
  corridorId: string;
}

export interface UseCorridorDetailReturn {
  data: CorridorDetailData | null;
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: CorridorDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

export interface CorridorDetailProps {
  corridorId?: string;
  onNavigateToCorridor?: (corridorId: string) => void;
  onGoBack?: () => void;
}

export const CORRIDOR_DETAIL_CACHE_PREFIX = 'corridor_detail:';
