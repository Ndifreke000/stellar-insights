import { describe, it, expect, vi, beforeEach } from "vitest";
import { PayRaider, PayRaiderError } from "../src/index.js";

// Minimal fetch mock
const mockFetch = vi.fn();
vi.stubGlobal("fetch", mockFetch);

function ok(body: unknown, headers: Record<string, string> = {}) {
  return Promise.resolve({
    ok: true,
    status: 200,
    json: () => Promise.resolve(body),
    headers: { get: (k: string) => headers[k] ?? null },
  });
}

function err(status: number, body: unknown) {
  return Promise.resolve({
    ok: false,
    status,
    statusText: "Error",
    json: () => Promise.resolve(body),
    headers: { get: () => null },
  });
}

describe("PayRaider SDK", () => {
  let client: PayRaider;

  beforeEach(() => {
    mockFetch.mockReset();
    client = new PayRaider({ apiKey: "test-key", maxRetries: 0 });
  });

  it("sends Bearer token", async () => {
    mockFetch.mockReturnValueOnce(ok({ data: [], pagination: {} }));
    await client.anchors.list();
    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect((init.headers as Record<string, string>)["Authorization"]).toBe("Bearer test-key");
  });

  it("appends pagination params to URL", async () => {
    mockFetch.mockReturnValueOnce(ok({ data: [], pagination: {} }));
    await client.anchors.list({ page: 2, limit: 10 });
    const [url] = mockFetch.mock.calls[0] as [string];
    expect(url).toContain("page=2");
    expect(url).toContain("limit=10");
  });

  it("throws PayRaiderError on 4xx", async () => {
    mockFetch.mockReturnValueOnce(
      err(401, { error: "UNAUTHORIZED", message: "Invalid API key", status: 401 }),
    );
    await expect(client.anchors.list()).rejects.toThrow(PayRaiderError);
  });

  it("throws with correct status and code", async () => {
    mockFetch.mockReturnValueOnce(
      err(404, { error: "NOT_FOUND", message: "Anchor not found", status: 404 }),
    );
    try {
      await client.anchors.get("missing");
    } catch (e) {
      expect(e).toBeInstanceOf(PayRaiderError);
      expect((e as PayRaiderError).status).toBe(404);
      expect((e as PayRaiderError).code).toBe("NOT_FOUND");
    }
  });

  it("retries on 429 up to maxRetries", async () => {
    const clientWithRetry = new PayRaider({
      apiKey: "test-key",
      maxRetries: 2,
      retryDelay: 0,
    });
    mockFetch
      .mockReturnValueOnce(err(429, { error: "RATE_LIMITED", message: "Too many requests", status: 429 }))
      .mockReturnValueOnce(err(429, { error: "RATE_LIMITED", message: "Too many requests", status: 429 }))
      .mockReturnValueOnce(ok({ data: [], pagination: {} }));

    const result = await clientWithRetry.anchors.list();
    expect(mockFetch).toHaveBeenCalledTimes(3);
    expect(result).toEqual({ data: [], pagination: {} });
  });

  it("posts body as JSON for cost estimate", async () => {
    mockFetch.mockReturnValueOnce(ok({ routes: [] }));
    await client.costCalculator.estimate({
      source_asset: "USD:GXXX",
      destination_asset: "EUR:GYYY",
      amount: 100,
    });
    const [, init] = mockFetch.mock.calls[0] as [string, RequestInit];
    expect(JSON.parse(init.body as string)).toMatchObject({ amount: 100 });
  });
});
