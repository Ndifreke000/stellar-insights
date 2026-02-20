/**
 * API Client for Stellar Insights
 * Handles all API calls to the backend
 */
import { monitoring } from "./monitoring";

export const API_BASE_URL =
  process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080/api";

/**
 * Custom error class for API responses
 */
export class ApiError extends Error {
  status: number;
  data: unknown;

  constructor(status: number, message: string, data?: unknown) {
    super(message);
    this.status = status;
    this.data = data;
    this.name = "ApiError";
  }
}

/**
 * Shared fetch wrapper with consistent error handling and types
 */
async function fetchApi<T>(
  endpoint: string,
  options: RequestInit = {},
): Promise<T> {
  const url = endpoint.startsWith("http")
    ? endpoint
    : `${API_BASE_URL}${endpoint}`;

  const headers = {
    "Content-Type": "application/json",
    ...options.headers,
  };

  try {
    const startTime = performance.now();
    const response = await fetch(url, {
      ...options,
      headers,
    });
    const duration = performance.now() - startTime;

    // Track API performance
    monitoring.trackMetric("api-response-time", duration, {
      endpoint,
      status: response.status,
      method: options.method || "GET",
    });

    if (!response.ok) {
      let errorData;
      try {
        errorData = await response.json();
      } catch {
        // Fallback if response is not JSON
        errorData = { message: response.statusText };
      }
      throw new ApiError(
        response.status,
        errorData.message || `API error: ${response.status}`,
        errorData,
      );
    }

    // Handle 204 No Content
    if (response.status === 204) {
      return {} as T;
    }

    return await response.json();
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }

    // Check if this is a network error (backend not running)
    const isNetworkError =
      error instanceof TypeError &&
      (error.message.includes("Failed to fetch") ||
        error.message.includes("fetch is not defined") ||
        error.message.includes("Network request failed"));

    const message =
      error instanceof Error ? error.message : "An unexpected error occurred";

    // Only log non-network errors to avoid noise when backend is not running
    if (!isNetworkError) {
      console.error(`API Request Error [${url}]:`, error);
    }

    throw new ApiError(0, message);
  }
}

/**
 * API client object with common HTTP methods
 */
export const api = {
  get: <T>(endpoint: string, options?: RequestInit) =>
    fetchApi<T>(endpoint, { ...options, method: "GET" }),

  post: <T>(endpoint: string, body?: unknown, options?: RequestInit) =>
    fetchApi<T>(endpoint, {
      ...options,
      method: "POST",
      body: body ? JSON.stringify(body) : undefined,
    }),

  put: <T>(endpoint: string, body?: unknown, options?: RequestInit) =>
    fetchApi<T>(endpoint, {
      ...options,
      method: "PUT",
      body: body ? JSON.stringify(body) : undefined,
    }),

  delete: <T>(endpoint: string, options?: RequestInit) =>
    fetchApi<T>(endpoint, { ...options, method: "DELETE" }),
};

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
 * Fetch all anchors with their metrics
 */
export async function getAnchors(
  limit?: number,
  offset?: number,
): Promise<AnchorsResponse> {
  const params = new URLSearchParams();
  if (limit !== undefined) params.append("limit", limit.toString());
  if (offset !== undefined) params.append("offset", offset.toString());

  const queryString = params.toString();
  const endpoint = `/anchors${queryString ? `?${queryString}` : ""}`;

  return api.get<AnchorsResponse>(endpoint);
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

/**
 * Fetch detailed metrics for a single anchor
 */
export async function getAnchorDetail(
  address: string,
): Promise<AnchorDetailData> {
  return api.get<AnchorDetailData>(`/anchors/${address}`);
}

/**
 * Mock data generator for Anchor Details
 */
export function generateMockAnchorDetail(address: string): AnchorDetailData {
  const now = new Date();

  // Generate reliability history (last 30 days)
  const reliability_history: ReliabilityDataPoint[] = [];
  for (let i = 29; i >= 0; i--) {
    const date = new Date(now.getTime() - i * 24 * 60 * 60 * 1000);
    // score between 70 and 100 with some random fluctuation
    reliability_history.push({
      timestamp: date.toISOString().split("T")[0],
      score: 85 + Math.random() * 15 - (Math.random() > 0.8 ? 10 : 0),
    });
  }

  // Generate issued assets
  const assets: IssuedAsset[] = [
    {
      asset_code: "USDC",
      issuer: address,
      volume_24h_usd: 1250000,
      success_rate: 98.5,
      failure_rate: 1.5,
      total_transactions: 5400,
    },
    {
      asset_code: "EURC",
      issuer: address,
      volume_24h_usd: 450000,
      success_rate: 94.2,
      failure_rate: 5.8,
      total_transactions: 1200,
    },
  ];

  return {
    anchor: {
      id: address,
      name: "Simulated Anchor Inc.",
      stellar_account: address,
      reliability_score:
        reliability_history[reliability_history.length - 1].score,
      asset_coverage: 2,
      failure_rate: 2.1,
      total_transactions: 6600,
      successful_transactions: 6461,
      failed_transactions: 139,
      status: "Healthy",
    },
    issued_assets: assets,
    reliability_history,
    top_failure_reasons: [
      { reason: "Timeout awaiting response", count: 45 },
      { reason: "Insufficient liquidity", count: 23 },
      { reason: "Path payment failed", count: 12 },
    ],
    recent_failed_corridors: [
      {
        corridor_id: "USDC-PHP",
        timestamp: new Date(now.getTime() - 1000 * 60 * 15).toISOString(),
      },
      {
        corridor_id: "EURC-NGN",
        timestamp: new Date(now.getTime() - 1000 * 60 * 145).toISOString(),
      },
    ],
  };
}

export interface AnchorsResponse {
  anchors: AnchorMetrics[];
  total: number;
}

export interface ListAnchorsParams {
  limit?: number;
  offset?: number;
}

/**
 * Fetch anchors from the backend API
 */
export async function fetchAnchors(
  params?: ListAnchorsParams,
): Promise<AnchorsResponse> {
  const searchParams = new URLSearchParams();

  if (params?.limit) {
    searchParams.append("limit", params.limit.toString());
  }
  if (params?.offset) {
    searchParams.append("offset", params.offset.toString());
  }

  const url = `${API_BASE_URL}/anchors${searchParams.toString() ? `?${searchParams.toString()}` : ""}`;
  try {
    const response = await fetch(url, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
      cache: "no-store",
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(
        errorData.error || `HTTP error! status: ${response.status}`,
      );
    }

    const data: AnchorsResponse = await response.json();
    return data;
  } catch (error) {
    console.error("Error fetching anchors:", error);
    throw error;
  }
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

/**
 * Generate mock prediction data for development
 */
function generateMockPrediction(
  request: PredictionRequest,
): PredictionResponse {
  // Generate a base probability based on common corridors
  const commonCorridors: Record<string, number> = {
    "USDC-XLM": 0.95,
    "USDC-EURC": 0.92,
    "XLM-USDC": 0.94,
    "USDC-PHP": 0.88,
    "USDC-NGN": 0.82,
    "EUR-USD": 0.91,
  };

  const corridorKey = `${request.source_asset}-${request.destination_asset}`;
  const baseProb = commonCorridors[corridorKey] ?? 0.7 + Math.random() * 0.2;

  // Adjust based on amount (higher amounts = slightly lower success)
  const amountFactor = Math.max(0.85, 1 - (request.amount / 100000) * 0.1);

  // Adjust based on time (peak hours slightly better)
  const hour = parseInt(request.time_of_day.split(":")[0], 10);
  const timeFactor = hour >= 9 && hour <= 17 ? 1.02 : 0.98;

  const successProb = Math.min(0.99, baseProb * amountFactor * timeFactor);

  // Calculate confidence interval (narrower for higher probabilities)
  const spread = (1 - successProb) * 0.3 + 0.02;
  const lowerBound = Math.max(0, successProb - spread);
  const upperBound = Math.min(1, successProb + spread / 2);

  // Determine risk level
  const riskLevel: "low" | "medium" | "high" =
    successProb >= 0.85 ? "low" : successProb >= 0.65 ? "medium" : "high";

  // Generate recommendation
  const recommendations: Record<string, string> = {
    low: "High probability of success. Proceed with payment.",
    medium:
      "Moderate success rate. Consider splitting into smaller amounts or adjusting timing.",
    high: "Risk of failure is elevated. Consider alternative corridors or waiting for better conditions.",
  };

  // Generate alternative routes
  const alternativeRoutes: AlternativeRoute[] = [
    {
      source_asset: request.source_asset,
      destination_asset: request.destination_asset,
      via_asset: "XLM",
      estimated_success_rate: Math.min(0.99, successProb + 0.03),
      description: `Route via XLM for better liquidity`,
    },
    {
      source_asset: request.source_asset,
      destination_asset: "USDC",
      estimated_success_rate: 0.96,
      description: `Convert to USDC first, then swap to ${request.destination_asset}`,
    },
  ].filter((route) => route.estimated_success_rate > successProb);

  return {
    success_probability: successProb,
    confidence_interval: [lowerBound, upperBound],
    risk_level: riskLevel,
    recommendation: recommendations[riskLevel],
    alternative_routes: alternativeRoutes,
    model_version: "1.0.0",
  };
}

/**
 * Get payment success prediction
 */
export async function getPaymentPrediction(
  request: PredictionRequest,
): Promise<PredictionResponse> {
  try {
    // Try to call the backend API
    const corridorId = `${request.source_asset}-${request.destination_asset}`;
    const response = await api.get<{
      success_probability: number;
      confidence: number;
      risk_level: string;
      recommendation: string;
      model_version: string;
    }>(
      `/ml/predict?corridor=${encodeURIComponent(corridorId)}&amount_usd=${request.amount}`,
    );

    // Transform backend response to frontend format
    const successProb = response.success_probability;
    const spread = (1 - response.confidence) * 0.2;

    return {
      success_probability: successProb,
      confidence_interval: [
        Math.max(0, successProb - spread),
        Math.min(1, successProb + spread / 2),
      ],
      risk_level: response.risk_level as "low" | "medium" | "high",
      recommendation: response.recommendation,
      alternative_routes: [],
      model_version: response.model_version,
    };
  } catch {
    // Fall back to mock data if backend is unavailable
    console.info("Using mock prediction data (backend unavailable)");
    return generateMockPrediction(request);
  }
}
