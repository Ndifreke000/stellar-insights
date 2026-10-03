/**
 * API Client for PayRaider
 * Handles all API calls to the backend
 */
import { monitoring } from "../monitoring";
import { logger } from "@/lib/logger";
import { appendPageParams, type PaginatedResponse } from "./pagination";
import { AnchorMetrics, MuxedAccountAnalytics, PredictionRequest, PredictionResponse } from "./types";

import { config } from '@/config';
export const API_BASE_URL = config.apiUrl;

/**
 * Custom error class for API responses
 */
export class ApiError extends Error {
  status: number;
  data: unknown;
  /** Backend `X-Request-ID` — search the API logs for this to find the request. */
  requestId?: string;

  constructor(status: number, message: string, data?: unknown, requestId?: string) {
    super(message);
    this.status = status;
    this.data = data;
    this.requestId = requestId;
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
    monitoring.trackApiCall(
      endpoint,
      options.method || "GET",
      response.status,
      duration,
    );

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
        response.headers.get("x-request-id") ?? errorData.request_id ?? undefined,
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
      logger.error(`API Request Error [${url}]:`, error);
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

/**
 * Fetch all anchors with their metrics
 */
export async function getAnchors(
  limit?: number,
  cursor?: string,
): Promise<PaginatedResponse<AnchorMetrics>> {
  const query = appendPageParams(new URLSearchParams(), { limit, cursor }).toString();
  return api.get<PaginatedResponse<AnchorMetrics>>(`/anchors${query ? `?${query}` : ""}`);
}
/**
 * Fetch muxed account usage analytics from the backend
 */
export async function getMuxedAnalytics(
  limit?: number,
): Promise<MuxedAccountAnalytics> {
  const params = new URLSearchParams();
  if (limit != null) params.set("limit", String(limit));
  const q = params.toString();
  return api.get<MuxedAccountAnalytics>(`/analytics/muxed${q ? `?${q}` : ""}`);
}

interface PreflightCorridor {
  id: string;
  source_asset: string;
  destination_asset: string;
  success_rate: number;
  total_attempts: number;
  successful_payments: number;
  health_score: number;
}

interface PreflightApiResponse {
  decision: "proceed" | "caution" | "hold" | "unknown";
  summary: string;
  corridor: PreflightCorridor | null;
  alternatives: PreflightCorridor[];
}

/**
 * 95% Wilson score interval for an observed success proportion. Narrow when
 * many payments were observed, wide when few were.
 */
export function wilsonInterval(successes: number, total: number): [number, number] {
  if (total <= 0) return [0, 1];
  const z = 1.96;
  const p = successes / total;
  const denominator = 1 + (z * z) / total;
  const centre = (p + (z * z) / (2 * total)) / denominator;
  const margin = (z * Math.sqrt((p * (1 - p)) / total + (z * z) / (4 * total * total))) / denominator;
  return [Math.max(0, centre - margin), Math.min(1, centre + margin)];
}

const RISK_BY_DECISION: Record<PreflightApiResponse["decision"], PredictionResponse["risk_level"]> = {
  proceed: "low",
  caution: "medium",
  hold: "high",
  // No recent data is not a reason to pay; treat it as high risk.
  unknown: "high",
};

/**
 * Estimate a payment's chance of success from the backend's pre-payment
 * check: the corridor's observed success rate over recent payments, a Wilson
 * interval around it, and the check's decision as the risk level.
 *
 * This used to call an ML endpoint the backend never served and, when that
 * failed, silently showed locally generated random numbers.
 */
export async function getPaymentPrediction(
  request: PredictionRequest,
): Promise<PredictionResponse> {
  const params = new URLSearchParams({
    source_asset: request.source_asset,
    destination_asset: request.destination_asset,
  });
  if (Number.isFinite(request.amount) && request.amount > 0) {
    params.set("amount_usd", String(request.amount));
  }
  const response = await api.get<PreflightApiResponse>(`/preflight?${params}`);

  const corridor = response.corridor;
  const successes = corridor?.successful_payments ?? 0;
  const total = corridor?.total_attempts ?? 0;

  return {
    success_probability: corridor ? corridor.success_rate / 100 : 0,
    confidence_interval: wilsonInterval(successes, total),
    risk_level: RISK_BY_DECISION[response.decision],
    recommendation: response.summary,
    alternative_routes: response.alternatives.map((alt) => ({
      source_asset: alt.source_asset,
      destination_asset: alt.destination_asset,
      estimated_success_rate: alt.success_rate / 100,
      description: `${alt.id} (health ${alt.health_score.toFixed(0)} of 100)`,
    })),
    model_version: `observed success rate over ${total} recent payments`,
  };
}
