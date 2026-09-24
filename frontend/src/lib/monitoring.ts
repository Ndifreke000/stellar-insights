/**
 * Frontend Monitoring Utility
 * Handles tracking of performance metrics (Web Vitals, page loads, API latency)
 * and application errors, and ships them to the backend RUM endpoint.
 */
import { logger } from "@/lib/logger";

export interface Metric {
  name: string;
  value: number;
  path: string;
  timestamp: string;
  metadata?: Record<string, unknown>;
}

export interface AppError {
  message: string;
  stack?: string;
  path: string;
  timestamp: string;
  userAgent: string;
  metadata?: Record<string, unknown>;
}

/**
 * Performance budgets (Core Web Vitals "good" thresholds). Values in ms, CLS unitless.
 * Kept in sync with the backend budgets in `observability/frontend_metrics.rs`.
 */
export const PERFORMANCE_BUDGETS: Record<string, number> = {
  "web-vitals-lcp": 2500,
  "web-vitals-fid": 100,
  "web-vitals-inp": 200,
  "web-vitals-cls": 0.1,
  "web-vitals-fcp": 1800,
  "web-vitals-ttfb": 800,
  "page-load-time": 3000,
  "api-response-time": 1000,
};

const API_BASE_URL = (
  process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080"
).replace(/\/api\/?$/, "");
/** Backend RUM endpoint (POST ingests a batch, GET returns the summary). */
export const FRONTEND_METRICS_ENDPOINT = `${API_BASE_URL}/api/metrics/frontend`;

class Monitoring {
  private static instance: Monitoring;
  private metricsBuffer: Metric[] = [];
  private errorsBuffer: AppError[] = [];
  private readonly MAX_BUFFER_SIZE = 50;
  private readonly FLUSH_INTERVAL = 10000; // 10 seconds

  private constructor() {
    if (typeof window !== "undefined") {
      // Automatic flushing
      setInterval(() => this.flush(), this.FLUSH_INTERVAL);
      // Flush remaining data when the page is hidden/unloaded
      window.addEventListener("pagehide", () => this.flush());
      document.addEventListener("visibilitychange", () => {
        if (document.visibilityState === "hidden") this.flush();
      });
    }
  }

  public static getInstance(): Monitoring {
    if (!Monitoring.instance) {
      Monitoring.instance = new Monitoring();
    }
    return Monitoring.instance;
  }

  /**
   * Track a performance metric
   */
  public trackMetric(
    name: string,
    value: number,
    metadata?: Record<string, unknown>,
  ) {
    const metric: Metric = {
      name,
      value,
      path: typeof window !== "undefined" ? window.location.pathname : "server",
      timestamp: new Date().toISOString(),
      metadata,
    };

    logger.debug(`[Monitoring] Metric: ${name} = ${value}`, metadata);
    this.checkBudget(metric);
    this.metricsBuffer.push(metric);

    if (this.metricsBuffer.length >= this.MAX_BUFFER_SIZE) {
      this.flush();
    }
  }

  /**
   * Report an error
   */
  public reportError(
    error: Error | string,
    metadata?: Record<string, unknown>,
  ) {
    const errorObj: AppError = {
      message: typeof error === "string" ? error : error.message,
      stack: typeof error === "string" ? undefined : error.stack,
      path: typeof window !== "undefined" ? window.location.pathname : "server",
      timestamp: new Date().toISOString(),
      userAgent:
        typeof window !== "undefined" ? window.navigator.userAgent : "server",
      metadata,
    };

    logger.error(`[Monitoring] Error: ${errorObj.message}`, errorObj);
    this.errorsBuffer.push(errorObj);

    // Errors are often critical, so flush immediately or soon
    this.flush();
  }

  /**
   * Track the latency of an API call. `status` 0 means a network failure.
   */
  public trackApiCall(
    endpoint: string,
    method: string,
    status: number,
    durationMs: number,
  ) {
    // Strip query strings and IDs to keep the endpoint label low-cardinality
    const normalized = endpoint
      .split("?")[0]
      .replace(/^https?:\/\/[^/]+/, "")
      .replace(/\/[0-9a-f-]{16,}|\/G[A-Z2-7]{55}|\/\d+/g, "/:id");
    this.trackMetric("api-response-time", durationMs, {
      endpoint: normalized,
      method: method.toUpperCase(),
      status,
    });
    if (status === 0 || status >= 500) {
      this.trackMetric("api-error", 1, { endpoint: normalized, status });
    }
  }

  /**
   * Warn when a metric exceeds its performance budget
   */
  private checkBudget(metric: Metric) {
    const budget = PERFORMANCE_BUDGETS[metric.name];
    if (budget !== undefined && metric.value > budget) {
      logger.warn(
        `[Monitoring] Performance budget exceeded: ${metric.name} = ${metric.value.toFixed(2)} (budget ${budget}) on ${metric.path}`,
        metric.metadata,
      );
    }
  }

  /**
   * Flush buffers to the backend
   */
  private async flush() {
    if (this.metricsBuffer.length === 0 && this.errorsBuffer.length === 0) {
      return;
    }

    const metricsToFlush = [...this.metricsBuffer];
    const errorsToFlush = [...this.errorsBuffer];

    this.metricsBuffer = [];
    this.errorsBuffer = [];

    try {
      await fetch("/api/metrics/frontend", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ metrics: metricsToFlush, errors: errorsToFlush }),
        keepalive: true,
      });
    } catch (e) {
      logger.error("[Monitoring] Failed to flush metrics", e);
    }
  }

  /**
   * Get device and browser info
   */
  public getDeviceInfo() {
    if (typeof window === "undefined")
      return { browser: "server", os: "server", device: "server" };

    const ua = window.navigator.userAgent;
    let browser = "Unknown";
    let os = "Unknown";
    let device = "Desktop";

    if (ua.includes("Firefox")) browser = "Firefox";
    else if (ua.includes("Chrome")) browser = "Chrome";
    else if (ua.includes("Safari")) browser = "Safari";
    else if (ua.includes("Edge")) browser = "Edge";

    if (ua.includes("Windows")) os = "Windows";
    else if (ua.includes("Mac")) os = "macOS";
    else if (ua.includes("Linux")) os = "Linux";
    else if (ua.includes("Android")) os = "Android";
    else if (ua.includes("iOS")) os = "iOS";

    if (/Mobi|Android/i.test(ua)) device = "Mobile";
    else if (/Tablet|iPad/i.test(ua)) device = "Tablet";

    return { browser, os, device };
  }
}

export const monitoring = Monitoring.getInstance();
