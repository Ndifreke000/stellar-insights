import { describe, expect, it, vi, beforeEach } from "vitest";
import { RequestDeduplication, SDKError } from "../src/index.js";

describe("RequestDeduplication", () => {
  let dedup: RequestDeduplication;

  beforeEach(() => {
    dedup = new RequestDeduplication({ apiKey: "test-key" });
  });

  // ── execute ────────────────────────────────────────────────────────────────

  it("should execute successfully", async () => {
    const fetcher = vi.fn().mockResolvedValue({ id: 1 });
    const result = await dedup.execute({ requestKey: "req-1", fetcher });

    expect(result.success).toBe(true);
    expect(result.requestKey).toBe("req-1");
    expect(result.deduplicated).toBe(false);
    expect(result.data).toEqual({ id: 1 });
    expect(fetcher).toHaveBeenCalledTimes(1);
  });

  it("trims whitespace from requestKey", async () => {
    const fetcher = vi.fn().mockResolvedValue("ok");
    const result = await dedup.execute({ requestKey: "  req-2  ", fetcher });

    expect(result.requestKey).toBe("req-2");
  });

  it("deduplicates concurrent requests with the same key", async () => {
    let resolveFetcher!: (value: string) => void;
    const fetcher = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          resolveFetcher = resolve;
        }),
    );

    const first = dedup.execute({ requestKey: "dup-key", fetcher });
    const second = dedup.execute({ requestKey: "dup-key", fetcher });

    expect(fetcher).toHaveBeenCalledTimes(1);
    expect(dedup.isInFlight("dup-key")).toBe(true);

    resolveFetcher("shared-result");

    const [firstResult, secondResult] = await Promise.all([first, second]);

    expect(firstResult.deduplicated).toBe(false);
    expect(secondResult.deduplicated).toBe(true);
    expect(firstResult.data).toBe("shared-result");
    expect(secondResult.data).toBe("shared-result");
    expect(dedup.isInFlight("dup-key")).toBe(false);
  });

  it("runs fetcher again after the in-flight request completes", async () => {
    const fetcher = vi.fn().mockResolvedValueOnce("first").mockResolvedValueOnce("second");

    const first = await dedup.execute({ requestKey: "seq", fetcher });
    const second = await dedup.execute({ requestKey: "seq", fetcher });

    expect(first.data).toBe("first");
    expect(second.data).toBe("second");
    expect(first.deduplicated).toBe(false);
    expect(second.deduplicated).toBe(false);
    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it("bypasses deduplication when skipDeduplication is true", async () => {
    const fetcher = vi.fn().mockResolvedValue("data");

    await dedup.execute({ requestKey: "skip", fetcher, skipDeduplication: true });
    await dedup.execute({ requestKey: "skip", fetcher, skipDeduplication: true });

    expect(fetcher).toHaveBeenCalledTimes(2);
  });

  it("clears in-flight entry when fetcher rejects", async () => {
    const fetcher = vi.fn().mockRejectedValue(new Error("network error"));

    await expect(dedup.execute({ requestKey: "fail", fetcher })).rejects.toBeInstanceOf(SDKError);
    expect(dedup.isInFlight("fail")).toBe(false);
  });

  it("throws SDKError when requestKey is empty", async () => {
    await expect(dedup.execute({ requestKey: "", fetcher: vi.fn() })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when requestKey is whitespace only", async () => {
    await expect(dedup.execute({ requestKey: "   ", fetcher: vi.fn() })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when fetcher is missing", async () => {
    await expect(
      dedup.execute({ requestKey: "bad", fetcher: undefined as unknown as () => Promise<unknown> }),
    ).rejects.toBeInstanceOf(SDKError);
  });

  // ── buildKey ─────────────────────────────────────────────────────────────

  it("buildKey returns url when params are absent", () => {
    expect(RequestDeduplication.buildKey("/api/anchors")).toBe("/api/anchors");
  });

  it("buildKey produces stable keys regardless of param order", () => {
    const keyA = RequestDeduplication.buildKey("/api/anchors", { page: 1, limit: 10 });
    const keyB = RequestDeduplication.buildKey("/api/anchors", { limit: 10, page: 1 });

    expect(keyA).toBe(keyB);
  });

  it("buildKey throws SDKError when url is empty", () => {
    expect(() => RequestDeduplication.buildKey("")).toThrow(SDKError);
  });

  // ── metrics ──────────────────────────────────────────────────────────────

  it("tracks deduplication metrics", async () => {
    let resolveFetcher!: (value: number) => void;
    const fetcher = vi.fn(
      () =>
        new Promise<number>((resolve) => {
          resolveFetcher = resolve;
        }),
    );

    const first = dedup.execute({ requestKey: "metrics", fetcher });
    const second = dedup.execute({ requestKey: "metrics", fetcher });

    resolveFetcher(42);
    await Promise.all([first, second]);

    const metrics = dedup.getMetrics();
    expect(metrics.totalRequests).toBe(2);
    expect(metrics.deduplicationHits).toBe(1);
    expect(metrics.inFlightCount).toBe(0);
  });

  it("resetMetrics clears counters", async () => {
    const fetcher = vi.fn().mockResolvedValue("x");
    await dedup.execute({ requestKey: "reset", fetcher });

    dedup.resetMetrics();
    const metrics = dedup.getMetrics();

    expect(metrics.totalRequests).toBe(0);
    expect(metrics.deduplicationHits).toBe(0);
  });

  // ── clear / clearAll ─────────────────────────────────────────────────────

  it("clear removes an in-flight entry", async () => {
    const fetcher = vi.fn(
      () =>
        new Promise<string>((resolve) => {
          setTimeout(() => resolve("done"), 50);
        }),
    );

    void dedup.execute({ requestKey: "clear-me", fetcher });
    expect(dedup.isInFlight("clear-me")).toBe(true);

    const removed = dedup.clear("clear-me");
    expect(removed).toBe(true);
    expect(dedup.isInFlight("clear-me")).toBe(false);
  });

  it("clearAll removes all in-flight entries", async () => {
    const fetcher = vi.fn(() => new Promise<string>(() => undefined));

    void dedup.execute({ requestKey: "a", fetcher });
    void dedup.execute({ requestKey: "b", fetcher });

    dedup.clearAll();

    expect(dedup.getMetrics().inFlightCount).toBe(0);
  });

  // ── constructor ────────────────────────────────────────────────────────────

  it("works with no config argument", async () => {
    const instance = new RequestDeduplication();
    const result = await instance.execute({
      requestKey: "no-config",
      fetcher: vi.fn().mockResolvedValue(true),
    });

    expect(result.success).toBe(true);
  });
});
