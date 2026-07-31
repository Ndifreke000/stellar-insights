import type { Request, Response } from "express";
import { StreamableHTTPServerTransport } from "@modelcontextprotocol/sdk/server/streamableHttp.js";
import { createMcpExpressApp } from "@modelcontextprotocol/sdk/server/express.js";
import type { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";

/**
 * Stateless streamable-HTTP transport: a fresh McpServer + transport per
 * request, no session state kept between calls. Simpler and enough for tool
 * calls, which are independent request/response pairs anyway - the stdio
 * transport (index.ts default) remains the one to use for stateful/streaming
 * use cases like Claude Desktop.
 */
export function startHttpServer(buildServer: () => McpServer, port: number): void {
  const app = createMcpExpressApp();

  app.post("/mcp", async (req, res) => {
    const server = buildServer();
    try {
      const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined });
      await server.connect(transport);
      await transport.handleRequest(req, res, req.body);
      res.on("close", () => {
        transport.close();
        server.close();
      });
    } catch (err) {
      console.error("Error handling MCP request:", err);
      if (!res.headersSent) {
        res.status(500).json({ jsonrpc: "2.0", error: { code: -32603, message: "Internal server error" }, id: null });
      }
    }
  });

  const methodNotAllowed = (_req: Request, res: Response) => {
    res.writeHead(405).end(JSON.stringify({ jsonrpc: "2.0", error: { code: -32000, message: "Method not allowed." }, id: null }));
  };
  app.get("/mcp", methodNotAllowed);
  app.delete("/mcp", methodNotAllowed);

  app.listen(port, () => {
    console.error(`stellar-insights-mcp-server listening on http://localhost:${port}/mcp`);
  });
}
