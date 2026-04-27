/**
 * Frontend Monitoring Utility
 * Handles tracking of performance metrics and application errors.
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
   * Track API call latency
   */
  public trackApiLatency(endpoint: string, latencyMs: number) {
    this.trackMetric("api-latency", latencyMs, { endpoint });
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
