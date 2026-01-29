/**
 * Analytics API Client
 * Handles all analytics data fetching from the backend
 */

export interface CorridorAnalytics {
  corridor_key: string;
  asset_a_code: string;
  asset_a_issuer: string;
  asset_b_code: string;
  asset_b_issuer: string;
  success_rate: number;
  total_transactions: number;
  successful_transactions: number;
  failed_transactions: number;
  volume_usd: number;
  avg_settlement_latency_ms?: number;
  liquidity_depth_usd: number;
  date: string;
}

export interface LiquidityDataPoint {
  timestamp: string;
  liquidity_usd: number;
  corridor_key: string;
}

export interface TVLDataPoint {
  timestamp: string;
  tvl_usd: number;
}

export interface SettlementLatencyDataPoint {
  timestamp: string;
  median_latency_ms: number;
  p95_latency_ms: number;
  p99_latency_ms: number;
}

export interface AnalyticsMetrics {
  top_corridors: CorridorAnalytics[];
  liquidity_history: LiquidityDataPoint[];
  tvl_history: TVLDataPoint[];
  settlement_latency_history: SettlementLatencyDataPoint[];
  total_volume_usd: number;
  avg_success_rate: number;
  active_corridors: number;
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

export async function fetchAnalyticsMetrics(): Promise<AnalyticsMetrics> {
  try {
    const response = await fetch(`${API_BASE}/api/analytics`, {
      method: "GET",
      headers: {
        "Content-Type": "application/json",
      },
    });

    if (!response.ok) {
      throw new Error(`API error: ${response.status}`);
    }

    return response.json();
  } catch (error) {
    // Check if this is a network error (backend not running)
    const isNetworkError = error instanceof TypeError && 
      (error.message.includes('Failed to fetch') || 
       error.message.includes('fetch is not defined') ||
       error.message.includes('Network request failed'));

    // Only log non-network errors to avoid noise when backend is not running
    if (!isNetworkError) {
      console.error("Failed to fetch analytics metrics:", error);
    }
    
    // Return mock data as fallback - this is expected when backend isn't running
    return getMockAnalyticsData();
  }
}

export function getMockAnalyticsData(): AnalyticsMetrics {
  const now = new Date();
  const lastSevenDays = Array.from({ length: 7 }, (_, i) => {
    const date = new Date(now);
    date.setDate(date.getDate() - (6 - i));
    return date;
  });

  const corridors = [
    {
      corridor_key: "USDC:GBUQWP3BOUZX34LOCALEXAMPLE->PHP:PHPEXAMPLEISSUER",
      asset_a_code: "USDC",
      asset_a_issuer: "GBUQWP3BOUZX34LOCALEXAMPLE",
      asset_b_code: "PHP",
      asset_b_issuer: "PHPEXAMPLEISSUER",
      success_rate: 98.5,
      total_transactions: 2450,
      successful_transactions: 2411,
      failed_transactions: 39,
      volume_usd: 890000,
      avg_settlement_latency_ms: 2340,
      liquidity_depth_usd: 5600000,
      date: now.toISOString(),
    },
    {
      corridor_key: "USD:ISSUERA->EUR:ISSUERB",
      asset_a_code: "USD",
      asset_a_issuer: "ISSUERA",
      asset_b_code: "EUR",
      asset_b_issuer: "ISSUERB",
      success_rate: 99.1,
      total_transactions: 1890,
      successful_transactions: 1872,
      failed_transactions: 18,
      volume_usd: 720000,
      avg_settlement_latency_ms: 1850,
      liquidity_depth_usd: 4200000,
      date: now.toISOString(),
    },
    {
      corridor_key: "USDC:ISSUERC->SGD:ISSUERD",
      asset_a_code: "USDC",
      asset_a_issuer: "ISSUERC",
      asset_b_code: "SGD",
      asset_b_issuer: "ISSUERD",
      success_rate: 96.8,
      total_transactions: 1240,
      successful_transactions: 1201,
      failed_transactions: 39,
      volume_usd: 450000,
      avg_settlement_latency_ms: 3100,
      liquidity_depth_usd: 2800000,
      date: now.toISOString(),
    },
    {
      corridor_key: "EUR:ISSUERE->GBP:ISSUERF",
      asset_a_code: "EUR",
      asset_a_issuer: "ISSUERE",
      asset_b_code: "GBP",
      asset_b_issuer: "ISSUERF",
      success_rate: 97.8,
      total_transactions: 1560,
      successful_transactions: 1527,
      failed_transactions: 33,
      volume_usd: 580000,
      avg_settlement_latency_ms: 2100,
      liquidity_depth_usd: 3500000,
      date: now.toISOString(),
    },
    {
      corridor_key: "USDC:ISSUERG->JPY:ISSUERH",
      asset_a_code: "USDC",
      asset_a_issuer: "ISSUERG",
      asset_b_code: "JPY",
      asset_b_issuer: "ISSUERH",
      success_rate: 98.2,
      total_transactions: 980,
      successful_transactions: 963,
      failed_transactions: 17,
      volume_usd: 360000,
      avg_settlement_latency_ms: 2600,
      liquidity_depth_usd: 2100000,
      date: now.toISOString(),
    },
  ];

  // Generate historical data for liquidity
  const liquidityHistory: LiquidityDataPoint[] = lastSevenDays.flatMap((date) =>
    corridors.slice(0, 3).map((corridor) => ({
      timestamp: date.toISOString(),
      liquidity_usd: corridor.liquidity_depth_usd * (0.8 + Math.random() * 0.4),
      corridor_key: corridor.corridor_key,
    }))
  );

  // Generate TVL history
  const tvlHistory: TVLDataPoint[] = lastSevenDays.map((date) => ({
    timestamp: date.toISOString(),
    tvl_usd:
      16700000 + Math.random() * 2000000 - 1000000 + (date.getTime() - lastSevenDays[0].getTime()) * 50000,
  }));

  // Generate settlement latency history
  const settlementLatencyHistory: SettlementLatencyDataPoint[] = lastSevenDays.map((date) => ({
    timestamp: date.toISOString(),
    median_latency_ms: 2300 + Math.random() * 600 - 300,
    p95_latency_ms: 4200 + Math.random() * 900 - 450,
    p99_latency_ms: 5800 + Math.random() * 1200 - 600,
  }));

  const totalVolume = corridors.reduce((sum, c) => sum + c.volume_usd, 0);
  const avgSuccessRate =
    corridors.reduce((sum, c) => sum + c.success_rate, 0) / corridors.length;

  return {
    top_corridors: corridors,
    liquidity_history: liquidityHistory,
    tvl_history: tvlHistory,
    settlement_latency_history: settlementLatencyHistory,
    total_volume_usd: totalVolume,
    avg_success_rate: avgSuccessRate,
    active_corridors: corridors.length,
  };
}
