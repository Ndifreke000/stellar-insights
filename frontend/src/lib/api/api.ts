/**
 * API Client for Stellar Insights
 * Handles all API calls to the backend
 */
import { monitoring } from "../monitoring";
import { logger } from "@/lib/logger";
import { AnchorsResponse, MuxedAccountAnalytics, PredictionRequest, PredictionResponse, AlternativeRoute } from "./types";
import { form } from "framer-motion/client";

export const API_BASE_URL = process.env.NEXT_PUBLIC_API_URL || "http://127.0.0.1:8080";

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
    logger.info("Using mock prediction data (backend unavailable)");
    return generateMockPrediction(request);
  }
}
