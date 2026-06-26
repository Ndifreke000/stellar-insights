import { describe, it, expect, beforeAll, skipIf } from "vitest";
import { StellarInsights } from "../../src/index.js";

// Skip tests if testnet credentials are not set
const TESTNET_RPC_URL = process.env.TESTNET_RPC_URL;
const TESTNET_SECRET_KEY = process.env.TESTNET_SECRET_KEY;
const skipTestnet = !TESTNET_RPC_URL || !TESTNET_SECRET_KEY;

describe.skipIf(skipTestnet)("Testnet: Client Integration Tests", () => {
  let client: StellarInsights;

  beforeAll(() => {
    // Initialize SDK with testnet configuration
    client = new StellarInsights({
      baseUrl: TESTNET_RPC_URL || "https://testnet-api.stellarinsights.io",
      apiKey: TESTNET_SECRET_KEY,
    });
  });

  it("initializes client successfully", () => {
    expect(client).toBeDefined();
    expect(client.anchors).toBeDefined();
    expect(client.corridors).toBeDefined();
    expect(client.prices).toBeDefined();
  });

  it("retrieves network info", async () => {
    const networkInfo = await client.network.info();
    expect(networkInfo).toBeDefined();
    expect(networkInfo.network).toBeDefined();
    expect(networkInfo.passphrase).toBeDefined();
    expect(networkInfo.horizon_url).toBeDefined();
  });

  it("lists anchors", async () => {
    const response = await client.anchors.list({ limit: 5 });
    expect(response).toBeDefined();
    expect(response.data).toBeInstanceOf(Array);
    expect(response.pagination).toBeDefined();
  });

  it("retrieves price data", async () => {
    const prices = await client.prices.list();
    expect(prices).toBeDefined();
    expect(prices).toBeInstanceOf(Array);
    if (prices.length > 0) {
      expect(prices[0].asset).toBeDefined();
      expect(prices[0].price_usd).toBeDefined();
    }
  });

  it("retrieves available networks", async () => {
    const networks = await client.network.available();
    expect(networks).toBeInstanceOf(Array);
    expect(networks.length).toBeGreaterThan(0);
  });

  it("lists liquidity pools", async () => {
    const response = await client.liquidityPools.list({ limit: 10 });
    expect(response).toBeDefined();
    expect(response.data).toBeInstanceOf(Array);
    expect(response.pagination).toBeDefined();
  });

  it("lists governance proposals", async () => {
    const response = await client.governance.listProposals({ limit: 10 });
    expect(response).toBeDefined();
    expect(response.data).toBeInstanceOf(Array);
    expect(response.pagination).toBeDefined();
  });
});
