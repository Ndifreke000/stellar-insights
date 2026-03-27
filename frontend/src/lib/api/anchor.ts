import { isStellarAccountAddress } from "../address";
import { logger } from "../logger";
import { api, API_BASE_URL } from "./api";
import { AnchorDetailData, AnchorMetrics, IssuedAsset, ReliabilityDataPoint } from "./types";


/**
 * Fetch detailed metrics for a single anchor (by G-address, M-address, or anchor ID)
 */
export async function getAnchorDetail(
  address: string,
): Promise<AnchorDetailData> {
  const trimmed = address?.trim() ?? "";
  if (isStellarAccountAddress(trimmed)) {
    return api.get<AnchorDetailData>(
      `/anchors/account/${encodeURIComponent(trimmed)}`,
    );
  }
  return api.get<AnchorDetailData>(`/anchors/${trimmed}`);
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
    logger.error("Error fetching anchors:", error);
    throw error;
  }
}