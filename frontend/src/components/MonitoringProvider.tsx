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
      },
    );
  });

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
