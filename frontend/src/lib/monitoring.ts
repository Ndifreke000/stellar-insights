/**
 * Frontend Monitoring Utility
 * Handles tracking of performance metrics and application errors.
 */

export interface Metric {
  name: string;
  value: number;
  path: string;
  timestamp: string;
  metadata?: Record<string, any>;
}

export interface AppError {
  message: string;
  stack?: string;
  path: string;
  timestamp: string;
  userAgent: string;
  metadata?: Record<string, any>;
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
    metadata?: Record<string, any>,
  ) {
    const metric: Metric = {
      name,
      value,
      path: typeof window !== "undefined" ? window.location.pathname : "server",
      timestamp: new Date().toISOString(),
      metadata,
    };

    console.log(`[Monitoring] Metric: ${name} = ${value}`, metadata);
    this.metricsBuffer.push(metric);

    if (this.metricsBuffer.length >= this.MAX_BUFFER_SIZE) {
      this.flush();
    }
  }

  /**
   * Report an error
   */
  public reportError(error: Error | string, metadata?: Record<string, any>) {
    const errorObj: AppError = {
      message: typeof error === "string" ? error : error.message,
      stack: typeof error === "string" ? undefined : error.stack,
      path: typeof window !== "undefined" ? window.location.pathname : "server",
      timestamp: new Date().toISOString(),
      userAgent:
        typeof window !== "undefined" ? window.navigator.userAgent : "server",
      metadata,
    };

    console.error(`[Monitoring] Error: ${errorObj.message}`, errorObj);
    this.errorsBuffer.push(errorObj);

    // Errors are often critical, so flush immediately or soon
    this.flush();
  }

  /**
   * Flush buffers to the backend (mocked for now)
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
      // In a real implementation, this would be:
      // await axios.post('/api/monitoring/batch', { metrics: metricsToFlush, errors: errorsToFlush });

      // For now, we simulate persistence in local storage for the dashboard to read
      if (typeof window !== "undefined") {
        const storedMetrics = JSON.parse(
          localStorage.getItem("mon_metrics") || "[]",
        );
        const storedErrors = JSON.parse(
          localStorage.getItem("mon_errors") || "[]",
        );

        localStorage.setItem(
          "mon_metrics",
          JSON.stringify([...storedMetrics, ...metricsToFlush].slice(-100)),
        );
        localStorage.setItem(
          "mon_errors",
          JSON.stringify([...storedErrors, ...errorsToFlush].slice(-100)),
        );
      }
    } catch (e) {
      console.error("[Monitoring] Failed to flush metrics", e);
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
