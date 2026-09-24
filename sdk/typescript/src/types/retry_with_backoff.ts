export interface RetryWithBackoffParams<T = unknown> {
  /** Async operation to execute with retries */
  operation: () => Promise<T>;
  /** Maximum retry attempts after the initial try (default: 3) */
  maxRetries?: number;
  /** Initial backoff delay in ms (default: 500) */
  initialDelayMs?: number;
  /** Cap for backoff delay in ms */
  maxDelayMs?: number;
  /** Add random jitter to backoff delays (default: true) */
  jitter?: boolean;
  /** HTTP status codes that should trigger a retry */
  retryableStatuses?: number[];
  /** Optional custom retry predicate */
  shouldRetry?: (error: unknown, attempt: number) => boolean;
  /** Optional label for observability */
  operationKey?: string;
}

export interface RetryWithBackoffResult<T = unknown> {
  success: true;
  data: T;
  attempts: number;
  retried: boolean;
  totalDelayMs: number;
  operationKey?: string;
}

export interface RetryWithBackoffMetrics {
  totalExecutions: number;
  totalRetries: number;
  totalFailures: number;
}
