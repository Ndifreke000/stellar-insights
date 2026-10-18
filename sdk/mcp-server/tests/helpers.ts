import { createServer, type Server } from "node:http";
import type { AddressInfo } from "node:net";

export interface RecordedRequest {
  method: string;
  url: string;
  authorization?: string;
  body: string;
}

export interface FakeBackend {
  url: string;
  requests: RecordedRequest[];
  close: () => Promise<void>;
}

type Route = { status?: number; body: unknown };

/**
 * A real HTTP server standing in for the PayRaider backend, so tests exercise
 * the SDK's actual requests rather than a stubbed fetch.
 */
export async function startFakeBackend(routes: Record<string, Route>): Promise<FakeBackend> {
  const requests: RecordedRequest[] = [];
  const server: Server = createServer((req, res) => {
    const chunks: Buffer[] = [];
    req.on("data", (chunk: Buffer) => chunks.push(chunk));
    req.on("end", () => {
      const path = decodeURIComponent(new URL(req.url ?? "/", "http://localhost").pathname);
      requests.push({
        method: req.method ?? "",
        url: req.url ?? "",
        authorization: req.headers.authorization,
        body: Buffer.concat(chunks).toString("utf8"),
      });
      const route = routes[`${req.method} ${path}`];
      res.writeHead(route?.status ?? (route ? 200 : 404), { "content-type": "application/json" });
      res.end(JSON.stringify(route?.body ?? { error: "NOT_FOUND", message: "no such route" }));
    });
  });
  await new Promise<void>((resolve) => server.listen(0, "127.0.0.1", resolve));
  const { port } = server.address() as AddressInfo;
  return {
    url: `http://127.0.0.1:${port}`,
    requests,
    close: () => new Promise((resolve) => server.close(() => resolve())),
  };
}

export const PREFLIGHT_PROCEED = {
  decision: "proceed",
  summary: "All checks passed; safe to pay on this corridor.",
  score: 92,
  corridor: null,
  checks: [],
  alternatives: [],
  evaluated_at: "2026-01-01T00:00:00Z",
};
