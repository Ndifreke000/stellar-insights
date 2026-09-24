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
      {
        label: metric.label,
        id: metric.id,
        rating: metric.rating,
        navigationType: metric.navigationType,
      },
    );
  });

  useEffect(() => {
    // Track full page load time from the Navigation Timing API
    const reportPageLoad = () => {
      const [nav] = performance.getEntriesByType(
        "navigation",
      ) as PerformanceNavigationTiming[];
      if (nav && nav.loadEventEnd > 0) {
        monitoring.trackMetric("page-load-time", nav.loadEventEnd, {
          domContentLoaded: nav.domContentLoadedEventEnd,
          transferSize: nav.transferSize,
        });
      }
    };
    const onLoad = () => setTimeout(reportPageLoad, 0);
    if (document.readyState === "complete") {
      onLoad();
    } else {
      window.addEventListener("load", onLoad, { once: true });
    }

    // Instrument fetch for API latency tracking
    const originalFetch = window.fetch;
    window.fetch = async function instrumentedFetch(input, init) {
      const url = typeof input === "string" ? input : input instanceof URL ? input.href : input.url;
      // Only track calls to our own API (not the metrics endpoint itself)
      if (url.startsWith("/api/") && !url.startsWith("/api/metrics/")) {
        const method = init?.method ?? (input instanceof Request ? input.method : "GET");
        const start = performance.now();
        try {
          const response = await originalFetch(input, init);
          monitoring.trackApiCall(url, method, response.status, Math.round(performance.now() - start));
          return response;
        } catch (err) {
          monitoring.trackApiCall(url, method, 0, Math.round(performance.now() - start));
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
      window.removeEventListener("load", onLoad);
      window.removeEventListener("error", handleError);
      window.removeEventListener("unhandledrejection", handleRejection);
    };
  }, []);

  return <>{children}</>;
}
