import type { PayRaiderConfig } from "./types.js";
import type {
  RequestDeduplicationMetrics,
  RequestDeduplicationParams,
  RequestDeduplicationResult,
} from "./types/request_deduplication.js";
import { SDKError } from "./sdk_error.js";

export class RequestDeduplication {
  private readonly config: PayRaiderConfig;
  private readonly inFlight = new Map<string, Promise<unknown>>();
  private totalRequests = 0;
  private deduplicationHits = 0;

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
  }

  /**
   * Execute a request with deduplication. Concurrent calls with the same
   * requestKey share a single in-flight promise.
   */
  public async execute<T>(params: RequestDeduplicationParams<T>): Promise<RequestDeduplicationResult<T>> {
    this.validateParams(params);

    const key = params.requestKey.trim();

    try {
      if (params.skipDeduplication) {
        this.totalRequests++;
        const data = await params.fetcher();
        return { success: true, requestKey: key, deduplicated: false, data };
      }

      const existing = this.inFlight.get(key);
      if (existing) {
        this.totalRequests++;
        this.deduplicationHits++;
        const data = (await existing) as T;
        return { success: true, requestKey: key, deduplicated: true, data };
      }

      this.totalRequests++;

      const promise = params.fetcher().finally(() => {
        this.inFlight.delete(key);
      });

      this.inFlight.set(key, promise);

      const data = await promise;
      return { success: true, requestKey: key, deduplicated: false, data };
    } catch (error) {
      if (error instanceof SDKError) throw error;
      throw new SDKError("Failed to execute deduplicated request", { params, requestKey: key }, { cause: error });
    }
  }

  /** Build a stable deduplication key from URL and params. */
  public static buildKey(url: string, params?: Record<string, unknown>): string {
    if (!url || !url.trim()) {
      throw new SDKError("url is required to build a deduplication key");
    }

    const normalizedUrl = url.trim();
    if (!params || Object.keys(params).length === 0) {
      return normalizedUrl;
    }

    const sortedParams = Object.keys(params)
      .sort()
      .reduce<Record<string, unknown>>((acc, paramKey) => {
        acc[paramKey] = params[paramKey];
        return acc;
      }, {});

    return `${normalizedUrl}:${JSON.stringify(sortedParams)}`;
  }

  /** Returns true when a request with the given key is currently in flight. */
  public isInFlight(requestKey: string): boolean {
    return this.inFlight.has(requestKey.trim());
  }

  /** Remove a pending in-flight entry without awaiting its result. */
  public clear(requestKey: string): boolean {
    return this.inFlight.delete(requestKey.trim());
  }

  /** Clear all tracked in-flight requests. */
  public clearAll(): void {
    this.inFlight.clear();
  }

  /** Snapshot of deduplication metrics for observability. */
  public getMetrics(): RequestDeduplicationMetrics {
    return {
      totalRequests: this.totalRequests,
      deduplicationHits: this.deduplicationHits,
      inFlightCount: this.inFlight.size,
    };
  }

  /** Reset deduplication metrics counters. */
  public resetMetrics(): void {
    this.totalRequests = 0;
    this.deduplicationHits = 0;
  }

  private validateParams<T>(params: RequestDeduplicationParams<T>): void {
    if (!params || typeof params.requestKey !== "string" || !params.requestKey.trim()) {
      throw new SDKError("requestKey is required", { params });
    }
    if (typeof params.fetcher !== "function") {
      throw new SDKError("fetcher must be a function", { params });
    }
  }
}
