#!/usr/bin/env node
import { z } from "zod";
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { createClientFromEnv, getSorobanConfigFromEnv, ALLOW_WRITES } from "./client.js";
import { READ_ONLY_TOOLS, type ToolDef } from "./tools.js";
import { WRITE_TOOLS } from "./write-tools.js";
import { registerResources } from "./resources.js";
import { readOnChainSnapshot } from "./soroban.js";
import type { StellarInsightsClient } from "./sdk-types.js";

function registerToolDef(server: McpServer, client: StellarInsightsClient, tool: ToolDef): void {
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

function registerVerifySnapshot(server: McpServer): void {
  server.registerTool(
    "verify_snapshot",
    {
      title: "verify_snapshot",
      description:
        "Read a Stellar Insights analytics snapshot hash directly from the on-chain Soroban contract " +
        "(tamper-proof source of truth - contract state can only change via a submit_snapshot transaction). " +
        "Omit epoch to read the latest snapshot. Pass expected_hash to check it against a hash you already have.",
      inputSchema: {
        epoch: z.number().int().positive().optional().describe("Epoch to look up; omitted = latest snapshot"),
        expected_hash: z.string().optional().describe("Hex-encoded hash to compare against the on-chain value"),
      },
    },
    async (args) => {
      try {
        const soroban = getSorobanConfigFromEnv();
        const onChain = await readOnChainSnapshot({ ...soroban, epoch: args.epoch as number | undefined });
        const expectedHash = args.expected_hash as string | undefined;
        const result = {
          ...onChain,
          matches: expectedHash ? expectedHash.toLowerCase() === onChain.hash.toLowerCase() : null,
        };
        return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        return { content: [{ type: "text", text: `Snapshot verification failed: ${message}` }], isError: true };
      }
    },
  );
}

export function buildServer(): McpServer {
  const client = createClientFromEnv();

  const server = new McpServer({
    name: "stellar-insights",
    version: "0.1.0",
  });

  for (const tool of READ_ONLY_TOOLS) {
    registerToolDef(server, client, tool);
  }

  registerVerifySnapshot(server);
  registerResources(server, client);

  if (ALLOW_WRITES) {
    for (const tool of WRITE_TOOLS) {
      registerToolDef(server, client, tool);
    }
  }

  return server;
}

async function main(): Promise<void> {
  if (process.env.STELLAR_INSIGHTS_MCP_TRANSPORT === "http") {
    const { startHttpServer } = await import("./http-server.js");
    const port = Number(process.env.STELLAR_INSIGHTS_MCP_HTTP_PORT ?? 3000);
    startHttpServer(buildServer, port);
    return;
  }

  const server = buildServer();
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err) => {
  console.error("stellar-insights-mcp-server failed to start:", err);
  process.exit(1);
});
