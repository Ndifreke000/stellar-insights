import { timingSafeEqual } from "node:crypto";
import {
  createServer,
  type IncomingMessage,
  type Server,
  type ServerResponse,
} from "node:http";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import type { ServerConfig } from "./config.js";
import { buildServer } from "./server.js";
import { VERSION } from "./version.js";

/** Largest request body accepted on the MCP endpoint. */
const MAX_BODY_BYTES = 1_000_000;

function sendJson(res: ServerResponse, status: number, body: unknown): void {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(body));
}

function rpcError(res: ServerResponse, status: number, message: string): void {
  sendJson(res, status, { jsonrpc: "2.0", error: { code: -32000, message }, id: null });
}

function bearerMatches(header: string | undefined, token: string): boolean {
  const presented = Buffer.from(header?.startsWith("Bearer ") ? header.slice(7) : "");
  const expected = Buffer.from(token);
  return presented.length === expected.length && timingSafeEqual(presented, expected);
}

async function readJsonBody(req: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  let size = 0;
  for await (const chunk of req) {
    size += (chunk as Buffer).length;
    if (size > MAX_BODY_BYTES) throw new RangeError("request body too large");
    chunks.push(chunk as Buffer);
  }
  return JSON.parse(Buffer.concat(chunks).toString("utf8"));
}

/**
 * Streamable-HTTP MCP endpoint for hosted deployments.
 *
 * Stateless: every POST gets its own server and transport, so instances can
 * sit behind a load balancer with no session affinity.
 *
 *   POST /mcp      MCP JSON-RPC
 *   GET  /healthz  liveness probe
 */
export function createHttpServer(config: ServerConfig): Server {
  return createServer((req, res) => {
    void handle(config, req, res).catch((err) => {
      console.error("payraider-mcp-server request failed:", err);
      if (!res.headersSent) rpcError(res, 500, "Internal server error");
    });
  });
}

async function handle(
  config: ServerConfig,
  req: IncomingMessage,
  res: ServerResponse,
): Promise<void> {
  const path = new URL(req.url ?? "/", "http://localhost").pathname;

  if (path === "/healthz") {
    sendJson(res, 200, { status: "ok", name: "payraider-mcp-server", version: VERSION });
    return;
  }

  if (path !== "/mcp") {
    sendJson(res, 404, { error: "not_found" });
    return;
  }

  // Browsers send Origin; reject ones that were not allow-listed so a web
  // page cannot drive a locally running server (DNS rebinding).
  const origin = req.headers.origin;
  if (origin) {
    if (!config.allowedOrigins.includes(origin)) {
      rpcError(res, 403, "Origin not allowed");
      return;
    }
    res.setHeader("access-control-allow-origin", origin);
    res.setHeader("vary", "Origin");
  }

  if (req.method === "OPTIONS") {
    res.writeHead(204, {
      "access-control-allow-methods": "POST, OPTIONS",
      "access-control-allow-headers": "authorization, content-type, mcp-protocol-version",
    });
    res.end();
    return;
  }

  if (config.authToken && !bearerMatches(req.headers.authorization, config.authToken)) {
    res.setHeader("www-authenticate", "Bearer");
    rpcError(res, 401, "Missing or invalid bearer token");
    return;
  }

  if (req.method !== "POST") {
    res.setHeader("allow", "POST, OPTIONS");
    rpcError(res, 405, "Method not allowed");
    return;
  }

  let body: unknown;
  try {
    body = await readJsonBody(req);
  } catch (err) {
    rpcError(res, err instanceof RangeError ? 413 : 400, "Invalid JSON request body");
    return;
  }

  const server = buildServer(config);
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
  res.on("close", () => {
    void transport.close();
    void server.close();
  });
  await server.connect(transport);
  await transport.handleRequest(req, res, body);
}

/** Start listening and resolve once the port is bound. */
export function listen(config: ServerConfig): Promise<Server> {
  const server = createHttpServer(config);
  return new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(config.port, config.host, () => resolve(server));
  });
}
