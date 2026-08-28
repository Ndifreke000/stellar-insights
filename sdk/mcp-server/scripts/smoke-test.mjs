import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");

// Verifies the MCP handshake and tool/resource listing work end to end.
// Uses a dummy API key since tools/list and resources/templates/list don't
// call the Stellar Insights API - only actually invoking a tool would.
const transport = new StdioClientTransport({
  command: "npx",
  args: ["tsx", "src/index.ts"],
  cwd: packageRoot,
  env: { ...process.env, STELLAR_INSIGHTS_API_KEY: process.env.STELLAR_INSIGHTS_API_KEY ?? "smoke-test-dummy-key" },
});

const client = new Client({ name: "smoke-test-client", version: "0.0.1" });
await client.connect(transport);

const tools = await client.listTools();
console.log(`Connected. Server exposes ${tools.tools.length} tools:`);
for (const t of tools.tools) console.log(` - ${t.name}`);

const resources = await client.listResourceTemplates();
console.log(`Resource templates: ${resources.resourceTemplates.map((r) => r.uriTemplate).join(", ")}`);

await client.close();
process.exit(0);
