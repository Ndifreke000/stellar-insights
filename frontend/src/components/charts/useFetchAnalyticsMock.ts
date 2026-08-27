"use client";

import { useEffect, useState } from "react";
import { fetchAnalyticsMetrics, AnalyticsMetrics } from "@/lib/analytics-api";
import { logger } from "@/lib/logger";

/**
 * Lightweight hook used by the chart-export gallery to fetch analytics data
 * without duplicating the full analytics page fetch logic.
 */
export function useFetchAnalyticsMock() {
  const [metrics, setMetrics] = useState<AnalyticsMetrics | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchAnalyticsMetrics()
      .then(setMetrics)
      .catch((err) => {
        const msg = err instanceof Error ? err.message : "Failed to load metrics";
        setError(msg);
        logger.error("ChartExportGallery: failed to load metrics", err);
      })
      .finally(() => setLoading(false));
  }, []);

  return { metrics, loading, error };
}
