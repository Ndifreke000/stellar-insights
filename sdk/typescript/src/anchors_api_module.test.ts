import { describe, expect, it, vi, beforeEach } from "vitest";
import { AnchorsAPIModule, SDKError } from "../src/index.js";
import * as resources from "../src/resources.js";
import type { Anchor } from "../src/types.js";

const mockAnchor: Anchor = {
  id: "anchor-1",
  name: "Test Anchor",
  account: "GABC123",
  status: "active",
  metrics: {
    total_transactions: 100,
    success_rate: 0.99,
    avg_transaction_time_ms: 250,
    total_volume_usd: 50000,
  },
};

const mockList = { data: [mockAnchor], pagination: { page: 1, limit: 20, total: 1, pages: 1 } };

describe("AnchorsAPIModule", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  // ── list ──────────────────────────────────────────────────────────────────

  it("lists anchors successfully", async () => {
    vi.spyOn(resources.AnchorsResource.prototype, "list").mockResolvedValue(mockList);

    const module = new AnchorsAPIModule({ apiKey: "key" });
    const result = await module.execute({ action: "list" });

    expect(result.success).toBe(true);
    expect(result.action).toBe("list");
    expect(result.data.anchors).toEqual([mockAnchor]);
    expect(result.data.total).toBe(1);
    expect(typeof result.data.executedAt).toBe("string");
  });

  it("passes pagination params to list", async () => {
    const spy = vi.spyOn(resources.AnchorsResource.prototype, "list").mockResolvedValue(mockList);

    const module = new AnchorsAPIModule();
    await module.execute({ action: "list", page: 2, limit: 10 });

    expect(spy).toHaveBeenCalledWith({ page: 2, limit: 10 });
  });

  // ── get ───────────────────────────────────────────────────────────────────

  it("gets a single anchor by id", async () => {
    vi.spyOn(resources.AnchorsResource.prototype, "get").mockResolvedValue(mockAnchor);

    const module = new AnchorsAPIModule({ apiKey: "key" });
    const result = await module.execute({ action: "get", anchorId: "anchor-1" });

    expect(result.success).toBe(true);
    expect(result.action).toBe("get");
    expect(result.data.anchor).toEqual(mockAnchor);
  });

  it("trims anchorId whitespace", async () => {
    const spy = vi.spyOn(resources.AnchorsResource.prototype, "get").mockResolvedValue(mockAnchor);

    const module = new AnchorsAPIModule();
    await module.execute({ action: "get", anchorId: "  anchor-1  " });

    expect(spy).toHaveBeenCalledWith("anchor-1");
  });

  // ── getByAccount ──────────────────────────────────────────────────────────

  it("gets an anchor by account", async () => {
    vi.spyOn(resources.AnchorsResource.prototype, "getByAccount").mockResolvedValue(mockAnchor);

    const module = new AnchorsAPIModule({ apiKey: "key" });
    const result = await module.execute({ action: "getByAccount", account: "GABC123" });

    expect(result.success).toBe(true);
    expect(result.action).toBe("getByAccount");
    expect(result.data.anchor).toEqual(mockAnchor);
  });

  it("trims account whitespace", async () => {
    const spy = vi.spyOn(resources.AnchorsResource.prototype, "getByAccount").mockResolvedValue(mockAnchor);

    const module = new AnchorsAPIModule();
    await module.execute({ action: "getByAccount", account: "  GABC123  " });

    expect(spy).toHaveBeenCalledWith("GABC123");
  });

  // ── validation ────────────────────────────────────────────────────────────

  it("throws SDKError for invalid action", async () => {
    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "invalid" as any })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when anchorId is missing for get", async () => {
    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "get" })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when anchorId is empty for get", async () => {
    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "get", anchorId: "  " })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when account is missing for getByAccount", async () => {
    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "getByAccount" })).rejects.toBeInstanceOf(SDKError);
  });

  it("throws SDKError when account is empty for getByAccount", async () => {
    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "getByAccount", account: "" })).rejects.toBeInstanceOf(SDKError);
  });

  it("wraps unexpected errors in SDKError", async () => {
    vi.spyOn(resources.AnchorsResource.prototype, "list").mockRejectedValue(new Error("network error"));

    const module = new AnchorsAPIModule();
    await expect(module.execute({ action: "list" })).rejects.toBeInstanceOf(SDKError);
  });

  it("works with no config argument", async () => {
    vi.spyOn(resources.AnchorsResource.prototype, "list").mockResolvedValue(mockList);
    const module = new AnchorsAPIModule();
    const result = await module.execute({ action: "list" });
    expect(result.success).toBe(true);
  });
});
