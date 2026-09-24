export type AnchorDataSource = 'live' | 'cache' | 'mock';

export interface AnchorMetrics {
  id: string;
  name: string;
  stellar_account: string;
  reliability_score: number;
  asset_coverage: number;
  failure_rate: number;
  total_transactions: number;
  successful_transactions: number;
  failed_transactions: number;
  status: string;
}

export interface IssuedAsset {
  asset_code: string;
  issuer: string;
  volume_24h_usd: number;
  success_rate: number;
  failure_rate: number;
  total_transactions: number;
}

export interface ReliabilityDataPoint {
  timestamp: string;
  score: number;
}

export interface FailureReason {
  reason: string;
  count: number;
}

export interface FailedCorridorRef {
  corridor_id: string;
  timestamp: string;
}

export interface AnchorDetailData {
  anchor: AnchorMetrics;
  issued_assets: IssuedAsset[];
  reliability_history: ReliabilityDataPoint[];
  top_failure_reasons?: FailureReason[];
  recent_failed_corridors?: FailedCorridorRef[];
}

export interface UseAnchorDetailOptions {
  anchorId: string;
}

export interface UseAnchorDetailReturn {
  data: AnchorDetailData | null;
  loading: boolean;
  error: string | null;
  warning: string | null;
  isOffline: boolean;
  dataSource: AnchorDataSource | null;
  isFromCache: boolean;
  refetch: () => Promise<void>;
}

export interface AnchorDetailProps {
  anchorId?: string;
  onGoBack?: () => void;
  onNavigateToCorridor?: (corridorId: string) => void;
}

export const ANCHOR_DETAIL_CACHE_PREFIX = 'anchor_detail:';
