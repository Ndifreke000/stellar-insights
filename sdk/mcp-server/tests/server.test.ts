import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { InMemoryTransport } from "@modelcontextprotocol/sdk/inMemory.js";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { buildServer, describeError } from "../src/server.js";
import { READ_ONLY_TOOLS, type ToolDef } from "../src/tools.js";
import { PREFLIGHT_PROCEED, startFakeBackend, type FakeBackend } from "./helpers.js";

const plain: ToolDef = { name: "list_corridors", description: "", schema: {}, call: async () => null };
const authed: ToolDef = { ...plain, name: "list_alert_history", requiresAuth: true };

describe("describeError", () => {
  it("tells a keyless caller that an API key is needed", () => {
    expect(describeError({ status: 401, message: "x" }, plain, false)).toContain("Set PAYRAIDER_API_KEY");
  });

  it("says the configured key was rejected when one is set", () => {
    expect(describeError({ status: 403, message: "x" }, plain, true)).toContain("was rejected");
  });

  it("explains that some tools need a user token rather than a key", () => {
    expect(describeError({ status: 401, message: "x" }, authed, true)).toContain("user access token");
  });

  it("distinguishes the free-tier rate limit from a keyed one", () => {
    expect(describeError({ status: 429 }, plain, false)).toContain("free anonymous rate limit");
    expect(describeError({ status: 429 }, plain, true)).toContain("for this API key");
  });

  it("keeps the API's own message for other failures", () => {
    expect(describeError({ status: 400, message: "amount_usd must be positive" }, plain, false)).toBe(
      "PayRaider API error (400): amount_usd must be positive",
    );
    expect(describeError(new Error("socket hang up"), plain, false)).toBe(
      "PayRaider API error: socket hang up",
    );
  });
});

describe("MCP server", () => {
  let backend: FakeBackend;
  let client: Client;

  beforeEach(async () => {
    backend = await startFakeBackend({
      "POST /api/v1/preflight": { body: PREFLIGHT_PROCEED },
      "GET /api/corridors/USDC:GA->NGN:GB": { body: { corridor: { id: "USDC:GA->NGN:GB" } } },
      "GET /api/alerts/history": { status: 401, body: { error: "Missing authentication token" } },
    });
    const server = buildServer({ baseUrl: backend.url });
    const [clientTransport, serverTransport] = InMemoryTransport.createLinkedPair();
    await server.connect(serverTransport);
    client = new Client({ name: "test", version: "0.0.0" });
    await client.connect(clientTransport);
  });

  afterEach(async () => {
    await client.close();
    await backend.close();
  });

  it("lists every tool as read-only", async () => {
    const { tools } = await client.listTools();
    expect(tools.map((t) => t.name)).toEqual(READ_ONLY_TOOLS.map((t) => t.name));
    expect(tools.every((t) => t.annotations?.readOnlyHint === true)).toBe(true);
  });

  it("returns the backend's decision from preflight_payment", async () => {
    const result = await client.callTool({
      name: "preflight_payment",
      arguments: { source_asset: "USDC", destination_asset: "NGN", amount_usd: 2500 },
    });
    const [content] = result.content as Array<{ type: string; text: string }>;
    expect(result.isError).toBeFalsy();
    expect(JSON.parse(content.text).decision).toBe("proceed");
  });

  it("rejects arguments that fail the input schema without calling the backend", async () => {
    const result = await client.callTool({
      name: "preflight_payment",
      arguments: { source_asset: "USDC", destination_asset: "NGN", amount_usd: -1 },
    });
    expect(result.isError).toBe(true);
    expect(backend.requests).toHaveLength(0);
  });

  it("reports an API failure as a tool error with guidance", async () => {
    const result = await client.callTool({ name: "list_alert_history", arguments: {} });
    const [content] = result.content as Array<{ type: string; text: string }>;
    expect(result.isError).toBe(true);
    expect(content.text).toContain("user access token");
  });

  it("serves a corridor through its resource URI", async () => {
    const { resourceTemplates } = await client.listResourceTemplates();
    expect(resourceTemplates.map((t) => t.uriTemplate)).toEqual([
      "payraider://corridor/{source}/{destination}",
      "payraider://anchor/{id}",
    ]);

    const read = await client.readResource({ uri: "payraider://corridor/USDC%3AGA/NGN%3AGB" });
    const [content] = read.contents as Array<{ uri: string; text: string }>;
    expect(JSON.parse(content.text).corridor.id).toBe("USDC:GA->NGN:GB");
  });
});
