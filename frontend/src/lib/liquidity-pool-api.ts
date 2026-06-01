/**
 * Liquidity Pool API Client
 * Handles all liquidity pool data fetching from the backend
 */

export interface LiquidityPool {
  pool_id: string;
  pool_type: string;
  fee_bp: number;
  total_trustlines: number;
  total_shares: string;
  reserve_a_asset_code: string;
  reserve_a_asset_issuer: string | null;
  reserve_a_amount: number;
  reserve_b_asset_code: string;
  reserve_b_asset_issuer: string | null;
  reserve_b_amount: number;
  total_value_usd: number;
  volume_24h_usd: number;
  fees_earned_24h_usd: number;
  apy: number;
  impermanent_loss_pct: number;
  trade_count_24h: number;
  last_synced_at: string;
  created_at: string;
  updated_at: string;
}

export interface PoolSnapshot {
  id: number;
  pool_id: string;
  reserve_a_amount: number;
  reserve_b_amount: number;
  total_value_usd: number;
  volume_usd: number;
  fees_usd: number;
  apy: number;
  impermanent_loss_pct: number;
  trade_count: number;
  snapshot_at: string;
}

export interface PoolStats {
  total_pools: number;
  total_value_locked_usd: number;
  total_volume_24h_usd: number;
  total_fees_24h_usd: number;
  avg_apy: number;
  avg_impermanent_loss: number;
}

export interface PoolDetailResponse {
  pool: LiquidityPool;
  snapshots: PoolSnapshot[];
}

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

async function safeFetch<T>(url: string, fallback: T): Promise<T> {
  try {
    const response = await fetch(url, {
      method: "GET",
      headers: { "Content-Type": "application/json" },
    });
    if (!response.ok) throw new Error(`API error: ${response.status}`);
    return response.json();
  } catch (error) {
    const isNetworkError =
      error instanceof TypeError &&
      (error.message.includes("Failed to fetch") ||
        error.message.includes("Network request failed"));
    if (!isNetworkError) {
      console.error(`Failed to fetch ${url}:`, error);
    }
    return fallback;
  }
}

export async function fetchPools(): Promise<LiquidityPool[]> {
  return safeFetch(`${API_BASE}/api/liquidity-pools/`, getMockPools());
}

export async function fetchPoolStats(): Promise<PoolStats> {
  return safeFetch(`${API_BASE}/api/liquidity-pools/stats`, getMockPoolStats());
}

export async function fetchPoolRankings(
  sortBy: string = "apy",
  limit: number = 20,
): Promise<LiquidityPool[]> {
  return safeFetch(
    `${API_BASE}/api/liquidity-pools/rankings?sort_by=${sortBy}&limit=${limit}`,
    getMockPools(),
  );
}

export async function fetchPoolDetail(
  poolId: string,
): Promise<PoolDetailResponse> {
  return safeFetch(
    `${API_BASE}/api/liquidity-pools/${poolId}`,
    getMockPoolDetail(poolId),
  );
}

export async function fetchPoolSnapshots(
  poolId: string,
  limit: number = 100,
): Promise<PoolSnapshot[]> {
  return safeFetch(
    `${API_BASE}/api/liquidity-pools/${poolId}/snapshots?limit=${limit}`,
    getMockSnapshots(poolId),
  );
}

// =============================================================================
// Mock Data
// =============================================================================

function getMockPools(): LiquidityPool[] {
  const now = new Date().toISOString();
  return [
    {
      pool_id: "pool_001",
      pool_type: "constant_product",
      fee_bp: 30,
      total_trustlines: 2450,
      total_shares: "850000.0",
      reserve_a_asset_code: "USDC",
      reserve_a_asset_issuer:
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      reserve_a_amount: 500000,
      reserve_b_asset_code: "XLM",
      reserve_b_asset_issuer: null,
      reserve_b_amount: 1200000,
      total_value_usd: 1700000,
      volume_24h_usd: 245000,
      fees_earned_24h_usd: 735,
      apy: 15.78,
      impermanent_loss_pct: 2.34,
      trade_count_24h: 892,
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      pool_id: "pool_002",
      pool_type: "constant_product",
      fee_bp: 30,
      total_trustlines: 1820,
      total_shares: "610000.0",
      reserve_a_asset_code: "USDC",
      reserve_a_asset_issuer:
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      reserve_a_amount: 320000,
      reserve_b_asset_code: "EURC",
      reserve_b_asset_issuer:
        "GDHU6WRG4IEQXM5NZ4BMPKOXHW76MZM4Y36DAVIZA67CE7BKBHP4V2OA",
      reserve_b_amount: 295000,
      total_value_usd: 615000,
      volume_24h_usd: 187000,
      fees_earned_24h_usd: 561,
      apy: 33.31,
      impermanent_loss_pct: 0.42,
      trade_count_24h: 654,
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      pool_id: "pool_003",
      pool_type: "constant_product",
      fee_bp: 30,
      total_trustlines: 980,
      total_shares: "750000.0",
      reserve_a_asset_code: "XLM",
      reserve_a_asset_issuer: null,
      reserve_a_amount: 450000,
      reserve_b_asset_code: "BTC",
      reserve_b_asset_issuer:
        "GDPJALI4AZKUU2W426U5WKMAT6CN3AJRPIIRYR2YM54TL2GDEMNQERFT",
      reserve_b_amount: 12.5,
      total_value_usd: 900012.5,
      volume_24h_usd: 98000,
      fees_earned_24h_usd: 294,
      apy: 11.93,
      impermanent_loss_pct: 5.12,
      trade_count_24h: 312,
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      pool_id: "pool_004",
      pool_type: "constant_product",
      fee_bp: 30,
      total_trustlines: 3200,
      total_shares: "360000.0",
      reserve_a_asset_code: "USDC",
      reserve_a_asset_issuer:
        "GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN",
      reserve_a_amount: 180000,
      reserve_b_asset_code: "yUSDC",
      reserve_b_asset_issuer:
        "GDGTVWSM4MGS2T7Z7GVZE5SAEVLSWM5SGY5Q2EMUQWRMEV2RNYY3YFG6",
      reserve_b_amount: 179500,
      total_value_usd: 359500,
      volume_24h_usd: 156000,
      fees_earned_24h_usd: 468,
      apy: 47.53,
      impermanent_loss_pct: 0.08,
      trade_count_24h: 1240,
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
    {
      pool_id: "pool_005",
      pool_type: "constant_product",
      fee_bp: 30,
      total_trustlines: 1560,
      total_shares: "420000.0",
      reserve_a_asset_code: "XLM",
      reserve_a_asset_issuer: null,
      reserve_a_amount: 800000,
      reserve_b_asset_code: "AQUA",
      reserve_b_asset_issuer:
        "GBNZILSTVQZ4R7IKQDGHYGY2QXL5QOFJYQMXPKWRRM5PAV7Y4M67AQUA",
      reserve_b_amount: 5000000,
      total_value_usd: 5800000,
      volume_24h_usd: 320000,
      fees_earned_24h_usd: 960,
      apy: 6.04,
      impermanent_loss_pct: 8.91,
      trade_count_24h: 478,
      last_synced_at: now,
      created_at: now,
      updated_at: now,
    },
  ];
}

function getMockPoolStats(): PoolStats {
  const pools = getMockPools();
  return {
    total_pools: pools.length,
    total_value_locked_usd: pools.reduce((s, p) => s + p.total_value_usd, 0),
    total_volume_24h_usd: pools.reduce((s, p) => s + p.volume_24h_usd, 0),
    total_fees_24h_usd: pools.reduce((s, p) => s + p.fees_earned_24h_usd, 0),
    avg_apy: pools.reduce((s, p) => s + p.apy, 0) / pools.length,
    avg_impermanent_loss:
      pools.reduce((s, p) => s + p.impermanent_loss_pct, 0) / pools.length,
  };
}

function getMockSnapshots(poolId: string): PoolSnapshot[] {
  const now = new Date();
  return Array.from({ length: 14 }, (_, i) => {
    const date = new Date(now);
    date.setDate(date.getDate() - (13 - i));
    return {
      id: i + 1,
      pool_id: poolId,
      reserve_a_amount: 500000 + Math.random() * 50000 - 25000,
      reserve_b_amount: 1200000 + Math.random() * 100000 - 50000,
      total_value_usd: 1700000 + Math.random() * 200000 - 100000,
      volume_usd: 200000 + Math.random() * 100000,
      fees_usd: 600 + Math.random() * 300,
      apy: 12 + Math.random() * 8,
      impermanent_loss_pct: 1 + Math.random() * 4,
      trade_count: 700 + Math.floor(Math.random() * 400),
      snapshot_at: date.toISOString(),
    };
  });
}

function getMockPoolDetail(poolId: string): PoolDetailResponse {
  const pools = getMockPools();
  const pool = pools.find((p) => p.pool_id === poolId) || pools[0];
  return {
    pool,
    snapshots: getMockSnapshots(poolId),
  };
}
