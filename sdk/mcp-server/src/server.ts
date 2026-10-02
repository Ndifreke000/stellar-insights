import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { createClient } from "./client.js";
import type { ServerConfig } from "./config.js";
import { registerResources } from "./resources.js";
import type { PayRaiderClient } from "./sdk-types.js";
import { READ_ONLY_TOOLS, type ToolDef } from "./tools.js";
import { VERSION } from "./version.js";

interface ApiErrorLike {
  status?: number;
  message?: string;
}

/**
 * Turn an SDK/API failure into a message an agent can act on, instead of a
 * bare status text.
 */
export function describeError(err: unknown, tool: ToolDef, hasApiKey: boolean): string {
  const { status, message } = (err ?? {}) as ApiErrorLike;
  const detail = message ?? String(err);

  if (status === 401 || status === 403) {
    if (tool.requiresAuth) {
      return `PayRaider API error (${status}): ${tool.name} needs a signed-in user access token; an API key alone is not enough.`;
    }
    return hasApiKey
      ? `PayRaider API error (${status}): the configured PAYRAIDER_API_KEY was rejected.`
      : `PayRaider API error (${status}): this endpoint needs an API key. Set PAYRAIDER_API_KEY.`;
  }
  if (status === 429) {
    return hasApiKey
      ? "PayRaider API error (429): rate limit reached for this API key. Retry shortly."
      : "PayRaider API error (429): the free anonymous rate limit was reached. Retry shortly, or set PAYRAIDER_API_KEY for a higher limit.";
  }
  if (status === 404) {
    return `PayRaider API error (404): not found. ${detail}`;
  }
  return status ? `PayRaider API error (${status}): ${detail}` : `PayRaider API error: ${detail}`;
}

/** Build an MCP server exposing the PayRaider tools and resources. */
export function buildServer(
  config: Pick<ServerConfig, "apiKey" | "baseUrl">,
  client: PayRaiderClient = createClient(config),
): McpServer {
  const server = new McpServer({ name: "payraider", version: VERSION });
  const hasApiKey = Boolean(config.apiKey);

  for (const tool of READ_ONLY_TOOLS) {
    server.registerTool(
      tool.name,
      {
        title: tool.name,
        description: tool.description,
        inputSchema: tool.schema,
        annotations: { readOnlyHint: true, openWorldHint: true },
      },
      async (args) => {
        try {
          const result = await tool.call(client, args as Record<string, unknown>);
          return { content: [{ type: "text", text: JSON.stringify(result, null, 2) }] };
        } catch (err) {
          return {
            content: [{ type: "text", text: describeError(err, tool, hasApiKey) }],
            isError: true,
          };
        }
      },
    );
  }

  registerResources(server, client);

  return server;
}
