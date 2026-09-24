import type { PayRaiderConfig } from "./types.js";
import type {
  RetryWithBackoffMetrics,
  RetryWithBackoffParams,
  RetryWithBackoffResult,
} from "./types/retry_with_backoff.js";
import { PayRaiderError } from "./http.js";
import { SDKError } from "./sdk_error.js";

const DEFAULT_MAX_RETRIES = 3;
const DEFAULT_INITIAL_DELAY_MS = 500;
const DEFAULT_MAX_DELAY_MS = 30_000;
const DEFAULT_RETRYABLE_STATUSES = [429, 500, 502, 503, 504];

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export class RetryWithBackoff {
  private readonly config: PayRaiderConfig;
  private totalExecutions = 0;
  private totalRetries = 0;
  private totalFailures = 0;

  constructor(config: PayRaiderConfig = {}) {
    this.config = config;
  }

  /**
   * Execute an operation with exponential backoff retries on transient failures.
   */
  public async execute<T>(params: RetryWithBackoffParams<T>): Promise<RetryWithBackoffResult<T>> {
    this.validateParams(params);

    const maxRetries = params.maxRetries ?? this.config.maxRetries ?? DEFAULT_MAX_RETRIES;
    const initialDelayMs = params.initialDelayMs ?? this.config.retryDelay ?? DEFAULT_INITIAL_DELAY_MS;
    const maxDelayMs = params.maxDelayMs ?? DEFAULT_MAX_DELAY_MS;
    const jitter = params.jitter ?? true;
    const retryableStatuses = new Set(params.retryableStatuses ?? DEFAULT_RETRYABLE_STATUSES);

    this.totalExecutions++;

    let attempt = 0;
    let totalDelayMs = 0;

    while (true) {
      try {
        const data = await params.operation();
        const retried = attempt > 0;

        if (retried) {
          this.totalRetries += attempt;
        }

        return {
          success: true,
          data,
          attempts: attempt + 1,
          retried,
          totalDelayMs,
          operationKey: params.operationKey?.trim() || undefined,
        };
      } catch (error) {
        const canRetry =
          attempt < maxRetries &&
          (params.shouldRetry?.(error, attempt) ??
            RetryWithBackoff.isRetryableError(error, retryableStatuses));

        if (!canRetry) {
          this.totalFailures++;
          if (error instanceof SDKError) throw error;
          throw new SDKError("Operation failed after retries", {
            params,
            attempts: attempt + 1,
            operationKey: params.operationKey,
          }, { cause: error });
        }

        const delayMs = RetryWithBackoff.calculateDelay(attempt, initialDelayMs, maxDelayMs, jitter);
        totalDelayMs += delayMs;
        attempt++;
        await sleep(delayMs);
      }
    }
  }

  /** Calculate exponential backoff delay with optional jitter. */
  public static calculateDelay(
    attempt: number,
    initialDelayMs: number,
    maxDelayMs = DEFAULT_MAX_DELAY_MS,
    jitter = true,
  ): number {
    const exponential = initialDelayMs * 2 ** attempt;
    const capped = Math.min(exponential, maxDelayMs);
    if (!jitter) return capped;
    return capped + Math.floor(Math.random() * Math.min(100, capped * 0.1));
  }

  /** Determine whether an error should be retried. */
  public static isRetryableError(
    error: unknown,
    retryableStatuses: Set<number> = new Set(DEFAULT_RETRYABLE_STATUSES),
  ): boolean {
    if (error instanceof TypeError) {
      return true;
    }

    if (error instanceof PayRaiderError) {
      if (retryableStatuses.has(error.status)) {
        return true;
      }
      if (error.status >= 400 && error.status < 500) {
        return false;
      }
      return error.status >= 500;
    }

    if (error instanceof SDKError) {
      const status = error.details?.status;
      if (typeof status === "number") {
        if (retryableStatuses.has(status)) return true;
        if (status >= 400 && status < 500) return false;
        return status >= 500;
      }
    }

    return false;
  }

  /** Snapshot of retry metrics for observability. */
  public getMetrics(): RetryWithBackoffMetrics {
    return {
      totalExecutions: this.totalExecutions,
      totalRetries: this.totalRetries,
      totalFailures: this.totalFailures,
    };
  }

  /** Reset retry metrics counters. */
  public resetMetrics(): void {
    this.totalExecutions = 0;
    this.totalRetries = 0;
    this.totalFailures = 0;
  }

  private validateParams<T>(params: RetryWithBackoffParams<T>): void {
    if (!params || typeof params.operation !== "function") {
      throw new SDKError("operation must be a function", { params });
    }
    if (params.maxRetries !== undefined && (typeof params.maxRetries !== "number" || params.maxRetries < 0)) {
      throw new SDKError("maxRetries must be a non-negative number", { params });
    }
    if (
      params.initialDelayMs !== undefined &&
      (typeof params.initialDelayMs !== "number" || params.initialDelayMs <= 0)
    ) {
      throw new SDKError("initialDelayMs must be a positive number", { params });
    }
    if (params.maxDelayMs !== undefined && (typeof params.maxDelayMs !== "number" || params.maxDelayMs <= 0)) {
      throw new SDKError("maxDelayMs must be a positive number", { params });
    }
  }
}
