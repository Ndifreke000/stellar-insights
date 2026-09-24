
/**
 * Analytics dashboard types and API client (GET /analytics/dashboard).
 */

import { ApiError, api } from "./api/api";

export interface NetworkVolumeDataPoint {
  time: string;
  volume: number;
  corridors: number;
  anchors: number;
}

export interface CorridorPerformanceMetric {
  corridor: string;
  success_rate: number;
  volume: number;
  health: number;
}

export interface NetworkStats {
  volume_24h: number;
  volume_growth: number;
  avg_success_rate: number;
  success_rate_growth: number;
  active_corridors: number;
  corridors_growth: number;
}

export interface AnalyticsDashboardData {
  stats: NetworkStats;
  time_series_data: NetworkVolumeDataPoint[];
  corridor_performance: CorridorPerformanceMetric[];
}

/** Raw JSON from backend may use integers for some fields; normalize for chart code. */
function normalizeDashboardData(raw: AnalyticsDashboardData): AnalyticsDashboardData {
  return {
    stats: {
      volume_24h: Number(raw.stats.volume_24h),
      volume_growth: Number(raw.stats.volume_growth),
      avg_success_rate: Number(raw.stats.avg_success_rate),
      success_rate_growth: Number(raw.stats.success_rate_growth),
      active_corridors: Number(raw.stats.active_corridors),
      corridors_growth: Number(raw.stats.corridors_growth),
    },
    time_series_data: (raw.time_series_data ?? []).map((p) => ({
      time: p.time,
      volume: Number(p.volume),
      corridors: Number(p.corridors),
      anchors: Number(p.anchors),
    })),
    corridor_performance: (raw.corridor_performance ?? []).map((c) => ({
      corridor: c.corridor,
      success_rate: Number(c.success_rate),
      volume: Number(c.volume),
      health: Number(c.health),
    })),
  };
}

/**
 * Load the analytics dashboard from the backend (no mock fallback).
 * @throws {ApiError} On HTTP error or network failure
 */
export async function getAnalyticsDashboard(): Promise<AnalyticsDashboardData> {
  const data = await api.get<AnalyticsDashboardData>("/analytics/dashboard");
  return normalizeDashboardData(data);
}

export type AnalyticsDashboardFetchResult =
  | { ok: true; data: AnalyticsDashboardData }
  | { ok: false; error: string; status?: number };

/**
 * Same as {@link getAnalyticsDashboard} but returns a result instead of throwing
 * (for UI loading / error states).
 */
export async function tryGetAnalyticsDashboard(): Promise<AnalyticsDashboardFetchResult> {
  try {
    const data = await getAnalyticsDashboard();
    return { ok: true, data };
  } catch (e) {
    const message =
      e instanceof ApiError
        ? e.message
        : e instanceof Error
          ? e.message
          : "Failed to load analytics dashboard";
    const status = e instanceof ApiError ? e.status : undefined;
    return { ok: false, error: message, status };
  }
}
