

/**
 * Anchor Detail Related Interfaces
 */

export interface IssuedAsset {
  asset_code: string;
  issuer: string;
  volume_24h_usd: number;
  success_rate: number;
  failure_rate: number;
  total_transactions: number;
}
/**
 * Prediction Request and Response Types
 */
export interface PredictionRequest {
  source_asset: string;
  destination_asset: string;
  amount: number;
  time_of_day: string;
}

export interface AlternativeRoute {
  source_asset: string;
  destination_asset: string;
  via_asset?: string;
  estimated_success_rate: number;
  description: string;
}

export interface PredictionResponse {
  success_probability: number;
  confidence_interval: [number, number];
  risk_level: "low" | "medium" | "high";
  recommendation: string;
  alternative_routes: AlternativeRoute[];
  model_version: string;
}

// =========================
// Muxed account analytics
// =========================

export interface MuxedAccountUsage {
  account_address: string;
  base_account: string | null;
  muxed_id: number | null;
  payment_count_as_source: number;
  payment_count_as_destination: number;
  total_payments: number;
}

/** Response fields match backend snake_case */
export type MuxedAccountAnalyticsResponse = MuxedAccountAnalytics;

export interface MuxedAccountAnalytics {
  total_muxed_payments: number;
  unique_muxed_addresses: number;
  top_muxed_by_activity: MuxedAccountUsage[];
  base_accounts_with_muxed: string[];
}


/**
 * Network-related types and functions
 */
export interface NetworkInfo {
  network: "mainnet" | "testnet";
  display_name: string;
  rpc_url: string;
  horizon_url: string;
  network_passphrase: string;
  color: string;
  is_mainnet: boolean;
  is_testnet: boolean;
}

export interface SwitchNetworkRequest {
  network: "mainnet" | "testnet";
}

export interface SwitchNetworkResponse {
  success: boolean;
  message: string;
  network_info: NetworkInfo;
}

export interface VolumeDataPoint {
  timestamp: string;
  volume_usd: number;
}

export interface SlippageDataPoint {
  timestamp: string;
  average_slippage_bps: number;
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

export interface AnchorsResponse {
  anchors: AnchorMetrics[];
  total: number;
}

export interface ReliabilityDataPoint {
  timestamp: string;
  score: number;
}

export interface AnchorDetailData {
  anchor: AnchorMetrics;
  issued_assets: IssuedAsset[];
  reliability_history: ReliabilityDataPoint[];
  top_failure_reasons?: { reason: string; count: number }[];
  recent_failed_corridors?: { corridor_id: string; timestamp: string }[];
}
