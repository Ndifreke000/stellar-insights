import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { StellarInsights } from "@stellar-insights/sdk";
import type { StellarInsightsClient } from "../src/sdk-types.js";
import { WRITE_TOOLS } from "../src/write-tools.js";

function mockFetchOnce(body: unknown, status = 200) {
  return vi.fn().mockResolvedValue({
    ok: status < 400,
    status,
    json: async () => body,
    headers: new Headers({ "content-type": "application/json" }),
  });
}

describe("WRITE_TOOLS", () => {
  let client: StellarInsightsClient;

  beforeEach(() => {
    client = new StellarInsights({ apiKey: "test-key", baseUrl: "https://example.test" });
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("registers every tool with a name, description and call function", () => {
    for (const tool of WRITE_TOOLS) {
      expect(tool.name).toMatch(/^[a-z_]+$/);
      expect(tool.description.length).toBeGreaterThan(10);
      expect(typeof tool.call).toBe("function");
    }
  });

  it("has no duplicate tool names, and none overlap with a read-only tool name", async () => {
    const { READ_ONLY_TOOLS } = await import("../src/tools.js");
    const writeNames = WRITE_TOOLS.map((t) => t.name);
    const readNames = new Set(READ_ONLY_TOOLS.map((t) => t.name));
    expect(new Set(writeNames).size).toBe(writeNames.length);
    for (const name of writeNames) {
      expect(readNames.has(name)).toBe(false);
    }
  });

  it("submit_transaction_envelope POSTs the envelope XDR", async () => {
    const fetchMock = mockFetchOnce({ id: "tx-1" });
    vi.stubGlobal("fetch", fetchMock);

    const tool = WRITE_TOOLS.find((t) => t.name === "submit_transaction_envelope")!;
    await tool.call(client, { envelope_xdr: "AAAA..." });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/transactions");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({ envelope_xdr: "AAAA..." });
  });

  it("vote_on_governance_proposal POSTs support to the proposal's vote endpoint", async () => {
    const fetchMock = mockFetchOnce({ for: 10, against: 2 });
    vi.stubGlobal("fetch", fetchMock);

    const tool = WRITE_TOOLS.find((t) => t.name === "vote_on_governance_proposal")!;
    await tool.call(client, { id: "prop-1", support: true });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/governance/proposals/prop-1/vote");
    expect(JSON.parse(init.body)).toEqual({ support: true });
  });

  it("delete_alert_rule DELETEs the rule by id", async () => {
    const fetchMock = mockFetchOnce(undefined, 204);
    vi.stubGlobal("fetch", fetchMock);

    const tool = WRITE_TOOLS.find((t) => t.name === "delete_alert_rule")!;
    await tool.call(client, { id: "rule-1" });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain("/api/alerts/rules/rule-1");
    expect(init.method).toBe("DELETE");
  });
});
