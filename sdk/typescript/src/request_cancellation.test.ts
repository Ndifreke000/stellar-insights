import { describe, expect, it, vi, beforeEach } from "vitest";
import { RequestCancellation, SDKError } from "../src/index.js";

describe("RequestCancellation", () => {
  let rc: RequestCancellation;

  beforeEach(() => {
    vi.useFakeTimers();
    rc = new RequestCancellation({ apiKey: "test-key" });
  });

  // ── execute ────────────────────────────────────────────────────────────────

  it("registers a new request and returns success", async () => {
    const result = await rc.execute({ requestKey: "req-1" });

    expect(result.success).toBe(true);
    expect(result.requestKey).toBe("req-1");
    expect(result.cancelled).toBe(false);
    expect(typeof result.data.registeredAt).toBe("string");
    expect(result.data.timeoutMs).toBeUndefined();
  });

  it("trims whitespace from requestKey", async () => {
    const result = await rc.execute({ requestKey: "  req-2  " });
    expect(result.requestKey).toBe("req-2");
  });

  it("cancels the previous request when the same key is re-registered", async () => {
    await rc.execute({ requestKey: "req-dup" });
    const signal = rc.getSignal("req-dup");

    const result = await rc.execute({ requestKey: "req-dup" });

    expect(result.cancelled).toBe(true);
    expect(signal?.aborted).toBe(true);
  });

  it("stores timeoutMs in result data", async () => {
    const result = await rc.execute({ requestKey: "req-timeout", timeoutMs: 5000 });
    expect(result.data.timeoutMs).toBe(5000);
  });

  it("auto-cancels after timeoutMs elapses", async () => {
    await rc.execute({ requestKey: "req-auto", timeoutMs: 1000 });
    const signal = rc.getSignal("req-auto");

    expect(signal?.aborted).toBe(false);
    vi.advanceTimersByTime(1000);
    expect(signal?.aborted).toBe(true);
  });

  it("throws SDKError when requestKey is empty", async () => {
    await expect(rc.execute({ requestKey: "" })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when requestKey is whitespace only", async () => {
    await expect(rc.execute({ requestKey: "   " })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when timeoutMs is zero", async () => {
    await expect(rc.execute({ requestKey: "req-bad", timeoutMs: 0 })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when timeoutMs is negative", async () => {
    await expect(rc.execute({ requestKey: "req-bad", timeoutMs: -100 })).rejects.toBeInstanceOf(SDKError);
  });

  // ── cancel ─────────────────────────────────────────────────────────────────

  it("cancel() aborts the signal and returns true", async () => {
    await rc.execute({ requestKey: "req-c" });
    const signal = rc.getSignal("req-c");

    const result = rc.cancel("req-c");

    expect(result).toBe(true);
    expect(signal?.aborted).toBe(true);
  });

  it("cancel() returns false for unknown key", () => {
    expect(rc.cancel("nonexistent")).toBe(false);
  });

  it("getSignal() returns undefined after cancel()", async () => {
    await rc.execute({ requestKey: "req-gone" });
    rc.cancel("req-gone");
    expect(rc.getSignal("req-gone")).toBeUndefined();
  });

  // ── getSignal ──────────────────────────────────────────────────────────────

  it("getSignal() returns an AbortSignal for a registered key", async () => {
    await rc.execute({ requestKey: "req-sig" });
    const signal = rc.getSignal("req-sig");

    expect(signal).toBeInstanceOf(AbortSignal);
    expect(signal?.aborted).toBe(false);
  });

  it("getSignal() returns undefined for an unknown key", () => {
    expect(rc.getSignal("unknown")).toBeUndefined();
  });

  // ── cancelAll ──────────────────────────────────────────────────────────────

  it("cancelAll() aborts all registered requests", async () => {
    await rc.execute({ requestKey: "a" });
    await rc.execute({ requestKey: "b" });
    const sigA = rc.getSignal("a");
    const sigB = rc.getSignal("b");

    rc.cancelAll();

    expect(sigA?.aborted).toBe(true);
    expect(sigB?.aborted).toBe(true);
    expect(rc.getSignal("a")).toBeUndefined();
    expect(rc.getSignal("b")).toBeUndefined();
  });

  // ── constructor ────────────────────────────────────────────────────────────

  it("works with no config argument", async () => {
    const instance = new RequestCancellation();
    const result = await instance.execute({ requestKey: "no-config" });
    expect(result.success).toBe(true);
  });
});
