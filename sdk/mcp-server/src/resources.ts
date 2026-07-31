import { McpServer, ResourceTemplate } from "@modelcontextprotocol/sdk/server/mcp.js";
import type { StellarInsightsClient } from "./sdk-types.js";

function asJsonContents(uri: URL, data: unknown) {
  return { contents: [{ uri: uri.href, mimeType: "application/json", text: JSON.stringify(data, null, 2) }] };
}

/**
 * Lets an agent (or a user in Claude) attach "this corridor" / "this anchor"
 * directly as context, mirroring the frontend's corridor/anchor detail pages,
 * instead of always going through a tool call.
 */
export function registerResources(server: McpServer, client: StellarInsightsClient): void {
  server.registerResource(
    "corridor",
    new ResourceTemplate("stellar-insights://corridor/{source}/{destination}", { list: undefined }),
    { title: "Stellar payment corridor", description: "Corridor detail: success rate, latency, liquidity trend" },
    async (uri, { source, destination }) => {
      const detail = await client.corridors.get(String(source), String(destination));
      return asJsonContents(uri, detail);
    },
  );

  server.registerResource(
    "anchor",
    new ResourceTemplate("stellar-insights://anchor/{id}", { list: undefined }),
    { title: "Stellar anchor operator", description: "Anchor detail: health score, supported assets, SEP compliance" },
    async (uri, { id }) => {
      const detail = await client.anchors.get(String(id));
      return asJsonContents(uri, detail);
    },
  );
}
