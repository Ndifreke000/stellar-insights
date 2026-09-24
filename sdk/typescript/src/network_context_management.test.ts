import { describe, expect, it, vi, beforeEach } from "vitest";
import { NetworkContextManagement, SDKError } from "../src/index.js";

describe("NetworkContextManagement", () => {
  let manager: NetworkContextManagement;

  beforeEach(() => {
    manager = new NetworkContextManagement({ apiKey: "test-key" });
  });

  // ── execute ────────────────────────────────────────────────────────────────

  it("should execute successfully", async () => {
    const result = await manager.execute({ clientName: "web-app" });

    expect(result.success).toBe(true);
    expect(result.network).toBe("mainnet");
    expect(result.data.baseUrl).toBe("https://api.payraider.io");
    expect(result.data.headers["X-Stellar-Network"]).toBe("mainnet");
    expect(result.data.clientName).toBe("web-app");
    expect(typeof result.data.configuredAt).toBe("string");
  });

  it("resolves testnet context when network override is provided", async () => {
    const result = await manager.execute({ network: "testnet" });

    expect(result.network).toBe("testnet");
    expect(result.data.baseUrl).toBe("https://testnet-api.payraider.io");
    expect(result.data.headers["X-Stellar-Network"]).toBe("testnet");
  });

  it("uses custom baseUrl from config", async () => {
    const custom = new NetworkContextManagement({ baseUrl: "https://custom.example.com" });
    const result = await custom.execute();

    expect(result.data.baseUrl).toBe("https://custom.example.com");
  });

  it("infers testnet from config baseUrl on construction", async () => {
    const testnetManager = new NetworkContextManagement({
      baseUrl: "https://testnet-api.payraider.io",
    });

    expect(testnetManager.getNetwork()).toBe("testnet");
  });

  it("throws SDKError for invalid network in execute params", async () => {
    await expect(
      manager.execute({ network: "invalid" as "mainnet" }),
    ).rejects.toBeInstanceOf(SDKError);
  });

  // ── setNetwork ─────────────────────────────────────────────────────────────

  it("setNetwork switches active network", () => {
    manager.setNetwork("testnet");
    expect(manager.getNetwork()).toBe("testnet");
  });

  it("setNetwork clears cache on switch", () => {
    manager.setCacheEntry("anchors", [{ id: "1" }]);
    manager.setNetwork("testnet");

    expect(manager.getCacheEntry("anchors")).toBeUndefined();
  });

  it("setNetwork emits change events", () => {
    const listener = vi.fn();
    manager.onNetworkChange(listener);

    manager.setNetwork("testnet");

    expect(listener).toHaveBeenCalledWith("testnet", "mainnet");
  });

  it("setNetwork does not emit when network is unchanged", () => {
    const listener = vi.fn();
    manager.onNetworkChange(listener);

    manager.setNetwork("mainnet");

    expect(listener).not.toHaveBeenCalled();
  });

  it("throws SDKError for invalid network in setNetwork", () => {
    expect(() => manager.setNetwork("devnet" as "mainnet")).toThrow(SDKError);
  });

  // ── getHeaders / getBaseUrl ────────────────────────────────────────────────

  it("getHeaders returns X-Stellar-Network header", () => {
    expect(manager.getHeaders()).toEqual({ "X-Stellar-Network": "mainnet" });
  });

  it("getHeaders accepts explicit network override", () => {
    expect(manager.getHeaders("testnet")).toEqual({ "X-Stellar-Network": "testnet" });
  });

  it("getBaseUrl returns network-specific default URL", () => {
    expect(manager.getBaseUrl("testnet")).toBe("https://testnet-api.payraider.io");
  });

  // ── cache ──────────────────────────────────────────────────────────────────

  it("setCacheEntry and getCacheEntry round-trip values", () => {
    manager.setCacheEntry("key", { value: 42 });
    expect(manager.getCacheEntry("key")).toEqual({ value: 42 });
  });

  it("throws SDKError when cache key is empty", () => {
    expect(() => manager.setCacheEntry("", "value")).toThrow(SDKError);
  });

  it("clearCache removes all entries", () => {
    manager.setCacheEntry("a", 1);
    manager.setCacheEntry("b", 2);
    manager.clearCache();

    expect(manager.getCacheEntry("a")).toBeUndefined();
    expect(manager.getCacheEntry("b")).toBeUndefined();
  });

  // ── onNetworkChange ────────────────────────────────────────────────────────

  it("onNetworkChange unsubscribe stops notifications", () => {
    const listener = vi.fn();
    const unsubscribe = manager.onNetworkChange(listener);

    unsubscribe();
    manager.setNetwork("testnet");

    expect(listener).not.toHaveBeenCalled();
  });

  it("throws SDKError when listener is not a function", () => {
    expect(() => manager.onNetworkChange(undefined as unknown as () => void)).toThrow(SDKError);
  });

  // ── validateNetwork ────────────────────────────────────────────────────────

  it("validateNetwork accepts mainnet and testnet", () => {
    expect(() => NetworkContextManagement.validateNetwork("mainnet")).not.toThrow();
    expect(() => NetworkContextManagement.validateNetwork("testnet")).not.toThrow();
  });

  it("validateNetwork rejects unknown values", () => {
    expect(() => NetworkContextManagement.validateNetwork("futurenet")).toThrow(SDKError);
  });

  // ── constructor ────────────────────────────────────────────────────────────

  it("works with no config argument", async () => {
    const instance = new NetworkContextManagement();
    const result = await instance.execute();

    expect(result.success).toBe(true);
    expect(result.network).toBe("mainnet");
  });
});
