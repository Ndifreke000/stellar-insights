import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { z } from "zod";
import { createClient } from "../src/client.js";
import type { PayRaiderClient } from "../src/sdk-types.js";
import { READ_ONLY_TOOLS } from "../src/tools.js";
import { PREFLIGHT_PROCEED, startFakeBackend, type FakeBackend } from "./helpers.js";

const tool = (name: string) => {
  const found = READ_ONLY_TOOLS.find((t) => t.name === name);
  if (!found) throw new Error(`no tool named ${name}`);
  return found;
};

describe("tool catalog", () => {
  it("gives every tool a snake_case name, a description and a call function", () => {
    for (const t of READ_ONLY_TOOLS) {
      expect(t.name).toMatch(/^[a-z_]+$/);
      expect(t.description.length).toBeGreaterThan(20);
      expect(typeof t.call).toBe("function");
    }
  });

  it("has no duplicate tool names", () => {
    const names = READ_ONLY_TOOLS.map((t) => t.name);
    expect(new Set(names).size).toBe(names.length);
  });

  it("lists the pre-payment check first", () => {
    expect(READ_ONLY_TOOLS[0].name).toBe("preflight_payment");
  });

  it("exposes no tool that writes", () => {
    const writes = READ_ONLY_TOOLS.filter((t) => /^(create|update|delete|submit|vote|revoke|rotate)_/.test(t.name));
    expect(writes).toEqual([]);
  });

  it("does not expose the unmounted ML endpoints", () => {
    const names = READ_ONLY_TOOLS.map((t) => t.name);
    expect(names).not.toContain("predict_payment_outcome");
    expect(names).not.toContain("get_ml_status");
  });
});

describe("tool input schemas", () => {
  const parse = (name: string, input: unknown) => z.object(tool(name).schema).safeParse(input);

  it("preflight_payment requires both assets and a positive amount", () => {
    expect(parse("preflight_payment", { source_asset: "USDC", destination_asset: "NGN" }).success).toBe(true);
    expect(parse("preflight_payment", { source_asset: "USDC" }).success).toBe(false);
    expect(
      parse("preflight_payment", { source_asset: "USDC", destination_asset: "NGN", amount_usd: -5 }).success,
    ).toBe(false);
    expect(
      parse("preflight_payment", { source_asset: "USDC", destination_asset: "NGN", min_success_rate: 140 }).success,
    ).toBe(false);
  });

  it("get_prices requires at least one asset", () => {
    expect(parse("get_prices", { assets: [] }).success).toBe(false);
    expect(parse("get_prices", { assets: ["XLM:native"] }).success).toBe(true);
  });

  it("list tools cap the page size at 200", () => {
    expect(parse("list_corridors", { limit: 201 }).success).toBe(false);
    expect(parse("list_corridors", { limit: 200 }).success).toBe(true);
  });
});

describe("tool requests", () => {
  let backend: FakeBackend;
  let client: PayRaiderClient;

  beforeEach(async () => {
    backend = await startFakeBackend({
      "POST /api/v1/preflight": { body: PREFLIGHT_PROCEED },
      "GET /api/corridors": { body: { data: [], pagination: {} } },
      "GET /api/corridors/USDC:GA->NGN:GB": { body: { corridor: { id: "USDC:GA->NGN:GB" } } },
      "GET /api/anchors/anchor 1": { body: { id: "anchor 1" } },
      "GET /api/prices": { body: { asset: "XLM:native", price_usd: 0.1 } },
      "GET /api/prices/batch": { body: { prices: {} } },
      "GET /api/prices/convert": { body: { usd: 1 } },
      "POST /api/cost-calculator/estimate": { body: { routes: [] } },
      "GET /api/network/info": { body: { network: "testnet" } },
      "GET /api/assets/verify/USDC/GA": { body: { asset_code: "USDC" } },
      "GET /api/assets/USDC/GA/verification": { body: { asset_code: "USDC" } },
      "GET /api/assets/verified": { body: { assets: [] } },
    });
    client = createClient({ baseUrl: backend.url });
  });

  afterEach(async () => {
    await backend.close();
  });

  const last = () => backend.requests[backend.requests.length - 1];

  it("preflight_payment POSTs only the fields that were given", async () => {
    const result = await tool("preflight_payment").call(client, {
      source_asset: "USDC",
      destination_asset: "NGN",
      amount_usd: 2500,
    });

    expect(last().method).toBe("POST");
    expect(last().url).toBe("/api/v1/preflight");
    expect(JSON.parse(last().body)).toEqual({
      source_asset: "USDC",
      destination_asset: "NGN",
      amount_usd: 2500,
    });
    expect(result).toEqual(PREFLIGHT_PROCEED);
  });

  it("sends no Authorization header without an API key", async () => {
    await tool("list_corridors").call(client, {});
    expect(last().authorization).toBeUndefined();
  });

  it("sends the API key as a bearer token when one is configured", async () => {
    const keyed = createClient({ baseUrl: backend.url, apiKey: "test-key" });
    await tool("list_corridors").call(keyed, { limit: 5 });
    expect(last().authorization).toBe("Bearer test-key");
    expect(last().url).toBe("/api/corridors?limit=5");
  });

  it("get_corridor addresses the corridor by its key", async () => {
    await tool("get_corridor").call(client, { corridor_key: "USDC:GA->NGN:GB" });
    expect(last().url).toBe("/api/corridors/USDC%3AGA-%3ENGN%3AGB");
  });

  it("get_anchor path-encodes the id", async () => {
    await tool("get_anchor").call(client, { id: "anchor 1" });
    expect(last().url).toBe("/api/anchors/anchor%201");
  });

  it("price tools use the query-string routes", async () => {
    await tool("get_price").call(client, { asset: "XLM:native" });
    expect(last().url).toBe("/api/prices?asset=XLM%3Anative");

    await tool("get_prices").call(client, { assets: ["XLM:native", "USDC:GA"] });
    expect(last().url).toBe("/api/prices/batch?assets=XLM%3Anative%2CUSDC%3AGA");

    await tool("convert_to_usd").call(client, { asset: "XLM:native", amount: 10 });
    expect(last().url).toBe("/api/prices/convert?asset=XLM%3Anative&amount=10");
  });

  it("estimate_transfer_cost sends the currency fields the API expects", async () => {
    await tool("estimate_transfer_cost").call(client, {
      source_currency: "USDC",
      destination_currency: "NGN",
      source_amount: 100,
    });
    expect(last().url).toBe("/api/cost-calculator/estimate");
    expect(JSON.parse(last().body)).toEqual({
      source_currency: "USDC",
      destination_currency: "NGN",
      source_amount: 100,
    });
  });

  it("network and asset tools hit the mounted routes", async () => {
    await tool("get_network_info").call(client, {});
    expect(last().url).toBe("/api/network/info");

    await tool("verify_asset").call(client, { asset_code: "USDC", asset_issuer: "GA" });
    expect(last().url).toBe("/api/assets/verify/USDC/GA");

    await tool("get_verified_asset").call(client, { asset_code: "USDC", asset_issuer: "GA" });
    expect(last().url).toBe("/api/assets/USDC/GA/verification");

    await tool("list_verified_assets").call(client, {});
    expect(last().url).toBe("/api/assets/verified");
  });

  it("rejects when the API answers with an error status", async () => {
    await expect(tool("get_liquidity_pool").call(client, { id: "missing" })).rejects.toMatchObject({
      status: 404,
    });
  });
});
