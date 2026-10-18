import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { RetryWithBackoff, SDKError, PayRaiderError } from "../src/index.js";

describe("RetryWithBackoff", () => {
  let retry: RetryWithBackoff;

  beforeEach(() => {
    vi.useFakeTimers();
    retry = new RetryWithBackoff({ maxRetries: 3, retryDelay: 100 });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  // ── execute ────────────────────────────────────────────────────────────────

  it("should execute successfully on first attempt", async () => {
    const operation = vi.fn().mockResolvedValue({ ok: true });

    const resultPromise = retry.execute({ operation, operationKey: "fetch-data" });
    await vi.runAllTimersAsync();
    const result = await resultPromise;

    expect(result.success).toBe(true);
    expect(result.data).toEqual({ ok: true });
    expect(result.attempts).toBe(1);
    expect(result.retried).toBe(false);
    expect(result.totalDelayMs).toBe(0);
    expect(operation).toHaveBeenCalledTimes(1);
  });

  it("retries on retryable errors and eventually succeeds", async () => {
    const operation = vi
      .fn()
      .mockRejectedValueOnce(new PayRaiderError(503, "UNAVAILABLE", "Service unavailable"))
      .mockRejectedValueOnce(new PayRaiderError(502, "BAD_GATEWAY", "Bad gateway"))
      .mockResolvedValue("success");

    const resultPromise = retry.execute({ operation, maxRetries: 3, initialDelayMs: 100 });
    await vi.runAllTimersAsync();
    const result = await resultPromise;

    expect(result.success).toBe(true);
    expect(result.data).toBe("success");
    expect(result.attempts).toBe(3);
    expect(result.retried).toBe(true);
    expect(result.totalDelayMs).toBeGreaterThan(0);
    expect(operation).toHaveBeenCalledTimes(3);
  });

  it("does not retry non-retryable 4xx errors", async () => {
    const operation = vi
      .fn()
      .mockRejectedValue(new PayRaiderError(404, "NOT_FOUND", "Not found"));

    await expect(retry.execute({ operation })).rejects.toBeInstanceOf(SDKError);
    expect(operation).toHaveBeenCalledTimes(1);
  });

  it("retries on 429 rate limit errors", async () => {
    const operation = vi
      .fn()
      .mockRejectedValueOnce(new PayRaiderError(429, "RATE_LIMITED", "Too many requests"))
      .mockResolvedValue("ok");

    const resultPromise = retry.execute({ operation, initialDelayMs: 50 });
    await vi.runAllTimersAsync();
    const result = await resultPromise;

    expect(result.data).toBe("ok");
    expect(result.attempts).toBe(2);
    expect(operation).toHaveBeenCalledTimes(2);
  });

  it("retries on network TypeError failures", async () => {
    const operation = vi
      .fn()
      .mockRejectedValueOnce(new TypeError("Failed to fetch"))
      .mockResolvedValue("recovered");

    const resultPromise = retry.execute({ operation, initialDelayMs: 50 });
    await vi.runAllTimersAsync();
    const result = await resultPromise;

    expect(result.data).toBe("recovered");
    expect(operation).toHaveBeenCalledTimes(2);
  });

  it("throws SDKError after exhausting max retries", async () => {
    const operation = vi
      .fn()
      .mockRejectedValue(new PayRaiderError(500, "SERVER_ERROR", "Server error"));

    const resultPromise = retry.execute({ operation, maxRetries: 2, initialDelayMs: 10 });
    const expectation = expect(resultPromise).rejects.toBeInstanceOf(SDKError);
    await vi.runAllTimersAsync();
    await expectation;
    expect(operation).toHaveBeenCalledTimes(3);
  });

  it("respects custom shouldRetry predicate", async () => {
    const operation = vi.fn().mockRejectedValue(new Error("custom failure"));
    const shouldRetry = vi.fn().mockReturnValue(false);

    await expect(retry.execute({ operation, shouldRetry })).rejects.toBeInstanceOf(SDKError);
    expect(shouldRetry).toHaveBeenCalledTimes(1);
    expect(operation).toHaveBeenCalledTimes(1);
  });

  it("throws SDKError when operation is missing", async () => {
    await expect(
      retry.execute({ operation: undefined as unknown as () => Promise<unknown> }),
    ).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when maxRetries is negative", async () => {
    await expect(
      retry.execute({ operation: vi.fn(), maxRetries: -1 }),
    ).rejects.toBeInstanceOf(SDKError);
  });

  // ── calculateDelay ─────────────────────────────────────────────────────────

  it("calculateDelay applies exponential backoff", () => {
    expect(RetryWithBackoff.calculateDelay(0, 100, 10_000, false)).toBe(100);
    expect(RetryWithBackoff.calculateDelay(1, 100, 10_000, false)).toBe(200);
    expect(RetryWithBackoff.calculateDelay(2, 100, 10_000, false)).toBe(400);
  });

  it("calculateDelay caps delay at maxDelayMs", () => {
    expect(RetryWithBackoff.calculateDelay(10, 100, 1000, false)).toBe(1000);
  });

  it("calculateDelay adds jitter when enabled", () => {
    const withJitter = RetryWithBackoff.calculateDelay(1, 100, 10_000, true);
    expect(withJitter).toBeGreaterThanOrEqual(200);
  });

  // ── isRetryableError ───────────────────────────────────────────────────────

  it("isRetryableError identifies retryable HTTP statuses", () => {
    expect(
      RetryWithBackoff.isRetryableError(
        new PayRaiderError(503, "UNAVAILABLE", "Unavailable"),
      ),
    ).toBe(true);
    expect(
      RetryWithBackoff.isRetryableError(new PayRaiderError(404, "NOT_FOUND", "Missing")),
    ).toBe(false);
    expect(RetryWithBackoff.isRetryableError(new TypeError("network"))).toBe(true);
  });

  // ── metrics ────────────────────────────────────────────────────────────────

  it("tracks retry metrics", async () => {
    const operation = vi
      .fn()
      .mockRejectedValueOnce(new PayRaiderError(500, "ERR", "fail"))
      .mockResolvedValue("done");

    const resultPromise = retry.execute({ operation, initialDelayMs: 10 });
    await vi.runAllTimersAsync();
    await resultPromise;

    const metrics = retry.getMetrics();
    expect(metrics.totalExecutions).toBe(1);
    expect(metrics.totalRetries).toBe(1);
    expect(metrics.totalFailures).toBe(0);
  });

  it("resetMetrics clears counters", async () => {
    const operation = vi.fn().mockResolvedValue("ok");
    const resultPromise = retry.execute({ operation });
    await vi.runAllTimersAsync();
    await resultPromise;

    retry.resetMetrics();
    expect(retry.getMetrics()).toEqual({
      totalExecutions: 0,
      totalRetries: 0,
      totalFailures: 0,
    });
  });

  // ── constructor ────────────────────────────────────────────────────────────

  it("works with no config argument", async () => {
    const instance = new RetryWithBackoff();
    const operation = vi.fn().mockResolvedValue(true);
    const resultPromise = instance.execute({ operation });
    await vi.runAllTimersAsync();
    const result = await resultPromise;

    expect(result.success).toBe(true);
  });
});
