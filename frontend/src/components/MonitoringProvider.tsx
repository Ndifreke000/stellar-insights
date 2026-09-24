"use client";

import React, { useEffect } from "react";
import { useReportWebVitals } from "next/web-vitals";
import { monitoring } from "@/lib/monitoring";

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
  // Track Web Vitals
  useReportWebVitals((metric) => {
    // Next.js Web Vitals: id, name, startTime, value, label
    monitoring.trackMetric(
      `web-vitals-${metric.name.toLowerCase()}`,
      metric.value,
      {
        label: metric.label,
        id: metric.id,
        rating: "rating" in metric ? metric.rating : undefined,
        navigationType:
          "navigationType" in metric ? metric.navigationType : undefined,
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
    if (document.readyState === "complete") {
      setTimeout(reportPageLoad, 0);
    } else {
      window.addEventListener("load", () => setTimeout(reportPageLoad, 0), {
        once: true,
      });
    }

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
