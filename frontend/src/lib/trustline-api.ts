import { API_BASE_URL } from "./api";

export interface TrustlineStat {
  asset_code: string;
  asset_issuer: string;
  total_trustlines: number;
  authorized_trustlines: number;
  unauthorized_trustlines: number;
  total_supply: number;
  created_at: string;
  updated_at: string;
}

export interface TrustlineSnapshot {
  id: number;
  asset_code: string;
  asset_issuer: string;
  total_trustlines: number;
  authorized_trustlines: number;
  unauthorized_trustlines: number;
  total_supply: number;
  snapshot_at: string;
}

export interface TrustlineMetrics {
  total_assets_tracked: number;
  total_trustlines_across_network: number;
  active_assets: number;
}

export async function fetchTrustlineStats(): Promise<TrustlineMetrics> {
  const url = `${API_BASE_URL}/trustlines/stats`;
  try {
    const res = await fetch(url, {
      next: { revalidate: 60 },
    });
    if (!res.ok) {
      throw new Error(`Failed to fetch trustline metrics: ${res.statusText}`);
    }
    return res.json();
  } catch (err) {
    console.error("Error fetching trustline stats:", err);
    return {
      total_assets_tracked: 0,
      total_trustlines_across_network: 0,
      active_assets: 0,
    };
  }
}

export async function fetchTrustlineRankings(
  limit: number = 20,
): Promise<TrustlineStat[]> {
  const url = `${API_BASE_URL}/trustlines/rankings?limit=${limit}`;
  try {
    const res = await fetch(url, {
      next: { revalidate: 60 },
    });
    if (!res.ok) {
      throw new Error(`Failed to fetch trustline rankings: ${res.statusText}`);
    }
    return res.json();
  } catch (err) {
    console.error("Error fetching trustline rankings:", err);
    return [];
  }
}

export async function fetchTrustlineHistory(
  assetCode: string,
  assetIssuer: string,
  limit: number = 30,
): Promise<TrustlineSnapshot[]> {
  const url = `${API_BASE_URL}/trustlines/${encodeURIComponent(
    assetCode,
  )}/${encodeURIComponent(assetIssuer)}/history?limit=${limit}`;
  try {
    const res = await fetch(url, {
      next: { revalidate: 300 },
    });
    if (!res.ok) {
      // Return empty on 404
      if (res.status === 404) return [];
      throw new Error(`Failed to fetch trustline history: ${res.statusText}`);
    }
    return res.json();
  } catch (err) {
    console.error("Error fetching trustline history:", err);
    return [];
  }
}
