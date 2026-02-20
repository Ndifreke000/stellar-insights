/**
 * Claimable Balance API Client
 * Handles all claimable balance data fetching from the backend
 */

export interface ClaimableBalance {
  id: string;
  asset_code: string;
  asset_issuer: string | null;
  amount: string;
  sponsor: string;
  created_at: string;
  expires_at: string | null;
  claimed: boolean;
  claimed_at: string | null;
  claimed_by: string | null;
  last_modified_ledger: number | null;
  claimant_count: number | null;
}

export interface ClaimableBalanceAnalytics {
  total_locked_count: number;
  pending_claims_count: number;
  expiring_soon_count: number;
  total_locked_value_usd: number;
  claim_success_rate: number;
  top_assets: TopAssetClaimable[];
}

export interface TopAssetClaimable {
  asset_code: string;
  asset_issuer: string | null;
  total_amount: number;
  count: number;
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

export async function fetchClaimableBalances(params?: {
  claimed?: boolean;
  asset_code?: string;
  limit?: number;
  offset?: number;
}): Promise<ClaimableBalance[]> {
  const search = new URLSearchParams();
  if (params?.claimed !== undefined) search.set("claimed", String(params.claimed));
  if (params?.asset_code) search.set("asset_code", params.asset_code);
  if (params?.limit) search.set("limit", String(params.limit));
  if (params?.offset) search.set("offset", String(params.offset));
  const qs = search.toString();
  return safeFetch(
    `${API_BASE}/api/claimable-balances${qs ? `?${qs}` : ""}`,
    [],
  );
}

export async function fetchClaimableBalance(id: string): Promise<ClaimableBalance | null> {
  try {
    const res = await fetch(`${API_BASE}/api/claimable-balances/${id}`);
    if (!res.ok) return null;
    return res.json();
  } catch {
    return null;
  }
}

export async function fetchExpiringBalances(days: number = 30): Promise<ClaimableBalance[]> {
  return safeFetch(
    `${API_BASE}/api/claimable-balances/expiring?days=${days}`,
    [],
  );
}

export async function fetchClaimableBalanceAnalytics(): Promise<ClaimableBalanceAnalytics> {
  return safeFetch(`${API_BASE}/api/claimable-balances/analytics`, {
    total_locked_count: 0,
    pending_claims_count: 0,
    expiring_soon_count: 0,
    total_locked_value_usd: 0,
    claim_success_rate: 0,
    top_assets: [],
  });
}
