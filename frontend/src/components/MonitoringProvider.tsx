"use client";

import React, { useEffect } from "react";
import { useReportWebVitals } from "next/web-vitals";
import { monitoring } from "@/lib/monitoring";
import { performanceProfiler } from "@/lib/performance";

/**
 * MonitoringProvider
 * - Tracks Web Vitals (LCP, FID, CLS, etc.)
 * - Listens for global runtime errors and unhandled rejections
 */
export function MonitoringProvider({
  children,
}: {
  children: React.ReactNode;
}) {
  // Track Web Vitals (LCP, FID, CLS, TTFB, INP)
  useReportWebVitals((metric) => {
    monitoring.trackMetric(
      `web-vitals-${metric.name.toLowerCase()}`,
      metric.value,
      { label: metric.label, id: metric.id },
    );
  });

  // Report navigation timing + initial memory snapshot once on mount
  useEffect(() => {
    const timing = performanceProfiler.getNavigationTiming();
    if (timing) {
      Object.entries(timing).forEach(([key, value]) => {
        monitoring.trackMetric(`nav:${key}`, value);
      });
    }

    const mem = performanceProfiler.snapshotMemory();
    if (mem) {
      monitoring.trackMetric('memory:usedJSHeapMB', mem.usedJSHeapMB, {
        totalJSHeapMB: mem.totalJSHeapMB,
        limitJSHeapMB: mem.limitJSHeapMB,
      });
    }
  }, []);

  useEffect(() => {
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
      window.removeEventListener("error", handleError);
      window.removeEventListener("unhandledrejection", handleRejection);
    };
  }, []);

  return <>{children}</>;
}
