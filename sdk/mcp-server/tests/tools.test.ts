import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { StellarInsights } from "@stellar-insights/sdk";
import type { StellarInsightsClient } from "../src/sdk-types.js";
import { READ_ONLY_TOOLS } from "../src/tools.js";

function mockFetchOnce(body: unknown, status = 200) {
  return vi.fn().mockResolvedValue({
    ok: status < 400,
    status,
    json: async () => body,
    headers: new Headers({ "content-type": "application/json" }),
  });
}

describe("READ_ONLY_TOOLS", () => {
  let client: StellarInsightsClient;

  beforeEach(() => {
    client = new StellarInsights({ apiKey: "test-key", baseUrl: "https://example.test" });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("registers every tool with a name, description and call function", () => {
    for (const tool of READ_ONLY_TOOLS) {
      expect(tool.name).toMatch(/^[a-z_]+$/);
      expect(tool.description.length).toBeGreaterThan(10);
      expect(typeof tool.call).toBe("function");
    }
  });

  it("has no duplicate tool names", () => {
    const names = READ_ONLY_TOOLS.map((t) => t.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("list_corridors calls GET /api/corridors", async () => {
    const fetchMock = mockFetchOnce({ data: [], total: 0, page: 1, limit: 20 });
    vi.stubGlobal("fetch", fetchMock);

    const tool = READ_ONLY_TOOLS.find((t) => t.name === "list_corridors")!;
    await tool.call(client, {});

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/corridors");
    expect(init.method).toBe("GET");
    expect(init.headers.Authorization).toBe("Bearer test-key");
  });

  it("get_anchor calls GET /api/anchors/:id with the id path-encoded", async () => {
    const fetchMock = mockFetchOnce({ id: "anchor-1" });
    vi.stubGlobal("fetch", fetchMock);

    const tool = READ_ONLY_TOOLS.find((t) => t.name === "get_anchor")!;
    await tool.call(client, { id: "anchor 1" });

    const [url] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/anchors/anchor%201");
  });

  it("estimate_transfer_cost POSTs the cost calculator body", async () => {
    const fetchMock = mockFetchOnce({ total_fee: 0.5 });
    vi.stubGlobal("fetch", fetchMock);

    const tool = READ_ONLY_TOOLS.find((t) => t.name === "estimate_transfer_cost")!;
    await tool.call(client, { source_asset: "USDC", destination_asset: "native", amount: 100 });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/cost-calculator/estimate");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({
      source_asset: "USDC",
      destination_asset: "native",
      amount: 100,
    });
  });

  it("surfaces API errors instead of throwing raw fetch errors", async () => {
    const fetchMock = mockFetchOnce({ error: "not_found", message: "corridor not found" }, 404);
    vi.stubGlobal("fetch", fetchMock);

    const tool = READ_ONLY_TOOLS.find((t) => t.name === "get_corridor")!;
    await expect(tool.call(client, { source: "USDC", destination: "native" })).rejects.toThrow();
  });
});
