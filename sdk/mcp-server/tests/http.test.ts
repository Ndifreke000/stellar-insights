import type { Server } from "node:http";
import type { AddressInfo } from "node:net";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { loadConfig } from "../src/config.js";
import { listen } from "../src/http.js";
import { VERSION } from "../src/version.js";
import { PREFLIGHT_PROCEED, startFakeBackend, type FakeBackend } from "./helpers.js";

const TOKEN = "test-bearer-token";
const INITIALIZE = JSON.stringify({
  jsonrpc: "2.0",
  id: 1,
  method: "initialize",
  params: { protocolVersion: "2025-03-26", capabilities: {}, clientInfo: { name: "t", version: "0" } },
});

describe("HTTP transport", () => {
  let backend: FakeBackend;
  let server: Server;
  let base: string;

  beforeEach(async () => {
    backend = await startFakeBackend({ "POST /api/v1/preflight": { body: PREFLIGHT_PROCEED } });
    // Port 0 asks the OS for a free port.
    const config = loadConfig({ PAYRAIDER_BASE_URL: backend.url }, ["--http"]);
    server = await listen({ ...config, port: 0, authToken: TOKEN, allowedOrigins: ["https://app.example"] });
    base = `http://127.0.0.1:${(server.address() as AddressInfo).port}`;
  });

  afterEach(async () => {
    await new Promise<void>((resolve) => server.close(() => resolve()));
    await backend.close();
  });

  const post = (headers: Record<string, string>, body = INITIALIZE) =>
    fetch(`${base}/mcp`, {
      method: "POST",
      headers: { "content-type": "application/json", accept: "application/json, text/event-stream", ...headers },
      body,
    });

  it("answers the liveness probe without a token", async () => {
    const res = await fetch(`${base}/healthz`);
    expect(res.status).toBe(200);
    expect(await res.json()).toEqual({ status: "ok", name: "payraider-mcp-server", version: VERSION });
  });

  it("returns 404 for unknown paths", async () => {
    expect((await fetch(`${base}/nope`)).status).toBe(404);
  });

  it("rejects a missing or wrong bearer token", async () => {
    expect((await post({})).status).toBe(401);
    expect((await post({ authorization: "Bearer wrong" })).status).toBe(401);
    expect((await post({ authorization: `Bearer ${TOKEN}x` })).status).toBe(401);
  });

  it("rejects an origin that is not allow-listed, and allows one that is", async () => {
    const denied = await post({ authorization: `Bearer ${TOKEN}`, origin: "https://evil.example" });
    expect(denied.status).toBe(403);

    const allowed = await post({ authorization: `Bearer ${TOKEN}`, origin: "https://app.example" });
    expect(allowed.status).toBe(200);
    expect(allowed.headers.get("access-control-allow-origin")).toBe("https://app.example");
  });

  it("only accepts POST on the MCP endpoint", async () => {
    const res = await fetch(`${base}/mcp`, { headers: { authorization: `Bearer ${TOKEN}` } });
    expect(res.status).toBe(405);
  });

  it("rejects a body that is not JSON", async () => {
    expect((await post({ authorization: `Bearer ${TOKEN}` }, "{not json")).status).toBe(400);
  });

  it("serves a full MCP session: list tools and run the pre-payment check", async () => {
    const transport = new StreamableHTTPClientTransport(new URL(`${base}/mcp`), {
      requestInit: { headers: { authorization: `Bearer ${TOKEN}` } },
    });
    const client = new Client({ name: "test", version: "0.0.0" });
    await client.connect(transport);

    const { tools } = await client.listTools();
    expect(tools[0].name).toBe("preflight_payment");

    const result = await client.callTool({
      name: "preflight_payment",
      arguments: { source_asset: "USDC", destination_asset: "NGN", amount_usd: 100 },
    });
    const [content] = result.content as Array<{ type: string; text: string }>;
    expect(JSON.parse(content.text).decision).toBe("proceed");
    expect(backend.requests.at(-1)?.url).toBe("/api/v1/preflight");

    await client.close();
  });
});
