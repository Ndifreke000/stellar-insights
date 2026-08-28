#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createClientFromEnv, ALLOW_WRITES } from "./client.js";
import { READ_ONLY_TOOLS } from "./tools.js";
import { registerResources } from "./resources.js";

export function buildServer(): McpServer {
  const client = createClientFromEnv();

  const server = new McpServer({
    name: "stellar-insights",
    version: "0.1.0",
  });

  for (const tool of READ_ONLY_TOOLS) {
    server.registerTool(
      tool.name,
      { title: tool.name, description: tool.description, inputSchema: tool.schema },
      async (args) => {
        try {
          const result = await tool.call(client, args as Record<string, unknown>);
          return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
        } catch (err) {
          const message = err instanceof Error ? err.message : String(err);
          return { content: [{ type: "text", text: `Stellar Insights API error: ${message}` }], isError: true };
        }
      },
    );
  }

  registerResources(server, client);

  if (ALLOW_WRITES) {
    // Write-scoped tools (transactions, governance votes, alert-rule and
    // webhook management) are a deliberately separate, opt-in module —
    // not yet implemented. Flip STELLAR_INSIGHTS_MCP_ALLOW_WRITES=true only
    // once that module exists; today this branch is a no-op by design.
  }

  return server;
}

async function main(): Promise<void> {
  const server = buildServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("stellar-insights-mcp-server failed to start:", err);
  process.exit(1);
});
