import { afterEach, describe, expect, it, vi } from "vitest";
import { PayRaider } from "./index.js";
import type { PreflightResponse } from "./types.js";

function stubFetch(body: unknown, status = 200) {
  const fetchMock = vi.fn().mockResolvedValue({
    ok: status < 400,
    status,
    json: async () => body,
    headers: new Headers({ "content-type": "application/json" }),
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

function response(decision: PreflightResponse["decision"]): PreflightResponse {
  return {
    decision,
    summary: "summary",
    score: decision === "unknown" ? null : 90,
    corridor: null,
    checks: [],
    alternatives: [],
    evaluated_at: "2026-01-01T00:00:00Z",
  };
}

describe("PreflightResource", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("POSTs the request body to /api/v1/preflight", async () => {
    const fetchMock = stubFetch(response("proceed"));
    const client = new PayRaider({ baseUrl: "https://example.test" });

    const result = await client.preflight.check({
      source_asset: "USDC",
      destination_asset: "NGN",
      amount_usd: 2500,
    });

    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toBe("https://example.test/api/v1/preflight");
    expect(init.method).toBe("POST");
    expect(JSON.parse(init.body)).toEqual({
      source_asset: "USDC",
      destination_asset: "NGN",
      amount_usd: 2500,
    });
    expect(result.decision).toBe("proceed");
  });

  it("works without an API key and sends no Authorization header", async () => {
    const fetchMock = stubFetch(response("proceed"));
    const client = new PayRaider({ baseUrl: "https://example.test" });

    await client.preflight.check({ source_asset: "USDC", destination_asset: "NGN" });

    const [, init] = fetchMock.mock.calls[0];
    expect(init.headers.Authorization).toBeUndefined();
  });

  it("isSafeToPay is true only for a proceed decision", async () => {
    const client = new PayRaider({ baseUrl: "https://example.test" });
    const req = { source_asset: "USDC", destination_asset: "NGN", amount_usd: 100 };

    stubFetch(response("proceed"));
    expect(await client.preflight.isSafeToPay(req)).toBe(true);

    for (const decision of ["caution", "hold", "unknown"] as const) {
      stubFetch(response(decision));
      expect(await client.preflight.isSafeToPay(req)).toBe(false);
    }
  });

  it("rejects when the API returns an error status", async () => {
    stubFetch({ error: "INVALID_AMOUNT", message: "amount_usd must be a positive number" }, 400);
    const client = new PayRaider({ baseUrl: "https://example.test" });

    await expect(
      client.preflight.check({ source_asset: "USDC", destination_asset: "NGN", amount_usd: -1 }),
    ).rejects.toThrow();
  });
});
