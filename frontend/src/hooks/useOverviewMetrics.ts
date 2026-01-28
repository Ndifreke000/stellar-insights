import { useQuery, useQueryClient } from "@tanstack/react-query";

// Types for the dashboard data
export interface Corridor {
  id: string;
  health: number;
  successRate: number;
}

export interface TopAsset {
  asset: string;
  volume: number;
  tvl: number;
}

export interface TimePoint {
  ts: string;
  successRate: number;
  settlementMs: number;
  tvl: number;
}

export interface DashboardData {
  totalSuccessRate: number;
  activeCorridors: Corridor[];
  topAssets: TopAsset[];
  timeseries: TimePoint[];
}

// Computed metrics with trend information
export interface TrendInfo {
  value: number;
  direction: "up" | "down" | "neutral";
  isGood: boolean;
}

export interface ComputedMetrics {
  successRate: {
    value: number;
    trend?: TrendInfo;
  };
  activeCorridors: {
    value: number;
  };
  tvl: {
    value: number;
    trend?: TrendInfo;
  };
  settlementTime: {
    value: number;
    trend?: TrendInfo;
  };
}

// Helper function to calculate trend
function getTrend(
  curr: number,
  prev: number,
  lowerIsBetter = false
): TrendInfo | undefined {
  if (!prev || prev === 0) return undefined;

  const diff = curr - prev;
  const pct = (diff / prev) * 100;

  let direction: "up" | "down" | "neutral" = "neutral";
  if (pct > 0.1) direction = "up";
  if (pct < -0.1) direction = "down";

  // Determine if "Good"
  // Normal: Up is good (Green), Down is bad (Red)
  // LowerIsBetter: Down is good (Green), Up is bad (Red)
  let isGood = true;
  if (lowerIsBetter) {
    isGood = direction === "down" || direction === "neutral";
  } else {
    isGood = direction === "up" || direction === "neutral";
  }

  return {
    value: pct,
    direction,
    isGood,
  };
}

// Compute metrics from raw data
function computeMetrics(data: DashboardData): ComputedMetrics | null {
  if (!data || !data.timeseries || data.timeseries.length === 0) return null;

  const current = data.timeseries[data.timeseries.length - 1];
  const previous = data.timeseries[0];

  return {
    successRate: {
      value: data.totalSuccessRate * 100,
      trend: getTrend(data.totalSuccessRate, previous.successRate / 100, false),
    },
    activeCorridors: {
      value: data.activeCorridors.length,
    },
    tvl: {
      value: current.tvl,
      trend: getTrend(current.tvl, previous.tvl, false),
    },
    settlementTime: {
      value: current.settlementMs,
      trend: getTrend(current.settlementMs, previous.settlementMs, true),
    },
  };
}

// Fetch function
async function fetchOverviewMetrics(): Promise<DashboardData> {
  const response = await fetch("/api/dashboard");

  if (!response.ok) {
    throw new Error(`HTTP ${response.status}: Failed to fetch metrics`);
  }

  return response.json();
}

// Main hook
export function useOverviewMetrics() {
  const queryClient = useQueryClient();

  const {
    data,
    isLoading,
    isError,
    error,
    isFetching,
    refetch,
  } = useQuery<DashboardData, Error>({
    queryKey: ["overviewMetrics"],
    queryFn: fetchOverviewMetrics,
  });

  // Compute derived metrics
  const metrics = data ? computeMetrics(data) : null;

  // Invalidate and refetch function for manual refresh
  const refresh = async () => {
    await queryClient.invalidateQueries({ queryKey: ["overviewMetrics"] });
    return refetch();
  };

  return {
    // Raw data
    data,
    // Computed metrics with trends
    metrics,
    // Loading states
    isLoading,
    isFetching,
    // Error states
    isError,
    error,
    // Actions
    refetch: refresh,
  };
}

// Export types for use in components
export type { DashboardData as OverviewMetricsData };
