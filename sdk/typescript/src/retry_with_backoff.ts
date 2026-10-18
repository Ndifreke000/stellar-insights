import type { PayRaiderConfig } from "./types.js";
import type {
  RetryWithBackoffMetrics,
  RetryWithBackoffParams,
  RetryWithBackoffResult,
} from "./types/retry_with_backoff.js";
import { PayRaiderError } from "./http.js";
import { SDKError } from "./sdk_error.js";
import {
  DEFAULT_BASE_DELAY_MS as DEFAULT_INITIAL_DELAY_MS,
  DEFAULT_MAX_DELAY_MS,
  DEFAULT_MAX_RETRIES,
  RETRYABLE_STATUSES,
  fullJitterDelay,
} from "./retry_policy.js";

const DEFAULT_RETRYABLE_STATUSES = [...RETRYABLE_STATUSES];

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

        const retryAfterMs = error instanceof PayRaiderError ? error.retryAfterMs : undefined;
        if (retryAfterMs !== undefined && retryAfterMs > maxDelayMs) {
          // Server asked us to wait longer than we are willing to; surface it.
          this.totalFailures++;
          throw new SDKError("Retry-After exceeds maxDelayMs", {
            params,
            attempts: attempt + 1,
            operationKey: params.operationKey,
          }, { cause: error });
        }
        const delayMs =
          retryAfterMs ?? RetryWithBackoff.calculateDelay(attempt, initialDelayMs, maxDelayMs, jitter);
        totalDelayMs += delayMs;
        attempt++;
        await sleep(delayMs);
      }
    }
  }

  /**
   * Calculate the backoff delay. With jitter (default) this is full jitter:
   * random(0, min(maxDelayMs, initialDelayMs * 2^attempt)). Without jitter it is
   * the capped exponential value.
   */
  public static calculateDelay(
    attempt: number,
    initialDelayMs: number,
    maxDelayMs = DEFAULT_MAX_DELAY_MS,
    jitter = true,
  ): number {
    if (jitter) return fullJitterDelay(attempt, initialDelayMs, maxDelayMs);
    return Math.min(initialDelayMs * 2 ** attempt, maxDelayMs);
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
      return retryableStatuses.has(error.status);
    }

    if (error instanceof SDKError) {
      const status = error.details?.status;
      if (typeof status === "number") return retryableStatuses.has(status);
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
