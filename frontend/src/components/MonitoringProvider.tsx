"use client";

import React, { useEffect } from "react";
import { useReportWebVitals } from "next/web-vitals";
import { monitoring } from "@/lib/monitoring";

/**
 * MonitoringProvider
 * - Tracks Web Vitals (LCP, FID, CLS, etc.)
 * - Instruments fetch to track API call latencies
 * - Listens for global runtime errors and unhandled rejections
 */
export function MonitoringProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  // Track Web Vitals
  useReportWebVitals((metric) => {
    monitoring.trackMetric(
      `web-vitals-${metric.name.toLowerCase()}`,
      metric.value,
      { label: metric.label, id: metric.id },
    );
  });

  useEffect(() => {
    // Instrument fetch for API latency tracking
    const originalFetch = window.fetch;
    window.fetch = async function instrumentedFetch(input, init) {
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      // Only track calls to our own API (not the metrics endpoint itself)
      if (url.startsWith("/api/") && !url.startsWith("/api/metrics/")) {
        const start = performance.now();
        try {
          const response = await originalFetch(input, init);
          monitoring.trackApiLatency(url, Math.round(performance.now() - start));
          return response;
        } catch (err) {
          monitoring.trackApiLatency(url, Math.round(performance.now() - start));
          throw err;
        }
      }
      return originalFetch(input, init);
    };

    // Track runtime errors
    const handleError = (event: ErrorEvent) => {
      monitoring.reportError(event.error || event.message, {
        filename: event.filename,
        lineno: event.lineno,
        colno: event.colno,
      });
    };

    // Track unhandled promise rejections
    const handleRejection = (event: PromiseRejectionEvent) => {
      monitoring.reportError(event.reason || "Unhandled Promise Rejection", {
        type: "promise_rejection",
      });
    };

    window.addEventListener("error", handleError);
    window.addEventListener("unhandledrejection", handleRejection);

    return () => {
      window.fetch = originalFetch;
      window.removeEventListener("error", handleError);
      window.removeEventListener("unhandledrejection", handleRejection);
    };
  }, []);

  return <>{children}</>;
}
