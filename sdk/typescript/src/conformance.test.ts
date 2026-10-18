// Runs the shared cases in sdk/conformance/cases against this SDK. The Python
// SDK runs the same files (sdk/python/tests/test_conformance.py); see
// sdk/conformance/README.md for the case format.
import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";
import { PayRaider, PayRaiderError } from "./index.js";
import type { PreflightRequest } from "./types.js";

const CASES_DIR = fileURLToPath(new URL("../../conformance/cases", import.meta.url));
const BASE_URL = "https://conformance.test";

interface ConformanceCase {
  name: string;
  description: string;
  client?: { api_key?: string };
  operation: string;
  input: Record<string, unknown>;
  request: {
    method: string;
    path: string;
    query: Record<string, string>;
    body: unknown;
    headers?: Record<string, string>;
  };
  response: { status: number; body: unknown };
  expect: { result?: unknown; error?: { status: number; code: string; message: string } };
}

type Input = Record<string, any>;

/** Neutral operation name -> this SDK's call. Keep in step with the README table. */
const OPERATIONS: Record<string, (c: PayRaider, i: Input) => Promise<unknown>> = {
  "preflight.check": (c, i) => c.preflight.check(i as PreflightRequest),
  "preflight.check_batch": (c, i) => c.preflight.checkBatch(i.items),
  "prices.get": (c, i) => c.prices.get(i.asset),
  "prices.batch": (c, i) => c.prices.batch(i.assets),
  "anchors.list": (c, i) => c.anchors.list({ page: i.page, limit: i.limit }),
  "anchors.get": (c, i) => c.anchors.get(i.id),
  "corridors.get": (c, i) => c.corridors.get(i.source, i.destination),
  "webhooks.delete": (c, i) => c.webhooks.delete(i.id),
  "network.info": (c) => c.network.info(),
};

const cases: ConformanceCase[] = readdirSync(CASES_DIR)
  .filter((f) => f.endsWith(".json"))
  .sort()
  .map((f) => JSON.parse(readFileSync(join(CASES_DIR, f), "utf8")));

interface Sent {
  method: string;
  path: string;
  query: Record<string, string>;
  body: unknown;
  headers: Record<string, string>;
}

function replay(response: ConformanceCase["response"]): Sent[] {
  const sent: Sent[] = [];
  vi.stubGlobal("fetch", async (input: string, init: RequestInit = {}) => {
    const url = new URL(input);
    const headers: Record<string, string> = {};
    for (const [k, v] of Object.entries((init.headers ?? {}) as Record<string, string>)) {
      headers[k.toLowerCase()] = v;
    }
    sent.push({
      method: init.method ?? "GET",
      path: url.pathname,
      query: Object.fromEntries(url.searchParams),
      body: typeof init.body === "string" ? JSON.parse(init.body) : null,
      headers,
    });
    const body = response.status === 204 ? null : JSON.stringify(response.body);
    return new Response(body, { status: response.status, headers: { "content-type": "application/json" } });
  });
  return sent;
}

describe("conformance", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("finds the shared cases", () => {
    expect(cases.length, `no cases found in ${CASES_DIR}`).toBeGreaterThan(0);
  });

  for (const tc of cases) {
    it(`${tc.name}: ${tc.description}`, async () => {
      const call = OPERATIONS[tc.operation];
      expect(call, `${tc.name}: operation "${tc.operation}" has no mapping in OPERATIONS`).toBeDefined();

      const sent = replay(tc.response);
      const client = new PayRaider({ baseUrl: BASE_URL, apiKey: tc.client?.api_key, maxRetries: 0 });

      let result: unknown;
      let error: unknown;
      try {
        result = await call(client, tc.input);
      } catch (err) {
        error = err;
      }

      expect(sent, `${tc.name}: expected exactly one HTTP request`).toHaveLength(1);
      const [req] = sent;
      expect(req.method, `${tc.name}: method`).toBe(tc.request.method);
      expect(req.path, `${tc.name}: path`).toBe(tc.request.path);
      expect(req.query, `${tc.name}: query`).toEqual(tc.request.query);
      expect(req.body, `${tc.name}: body`).toEqual(tc.request.body);
      for (const [name, value] of Object.entries(tc.request.headers ?? {})) {
        expect(req.headers[name], `${tc.name}: header ${name}`).toBe(value);
      }

      if (tc.expect.error) {
        expect(error, `${tc.name}: expected a PayRaiderError, got result ${JSON.stringify(result)}`).toBeInstanceOf(
          PayRaiderError,
        );
        const e = error as PayRaiderError;
        expect({ status: e.status, code: e.code, message: e.message }, `${tc.name}: error`).toEqual(tc.expect.error);
      } else {
        expect(error, `${tc.name}: unexpected error`).toBeUndefined();
        expect(result ?? null, `${tc.name}: result`).toEqual(tc.expect.result);
      }
    });
  }
});
