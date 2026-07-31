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

console.log("\nCalling verify_snapshot (live testnet contract call, no epoch = latest)...");
const verifyResult = await client.callTool({ name: "verify_snapshot", arguments: {} });
console.log(JSON.stringify(verifyResult, null, 2));

await client.close();

console.log("\nConnecting a second client with STELLAR_INSIGHTS_MCP_ALLOW_WRITES=true...");
const writeTransport = new StdioClientTransport({
  command: "npx",
  args: ["tsx", "src/index.ts"],
  cwd: packageRoot,
  env: {
    ...process.env,
    STELLAR_INSIGHTS_API_KEY: process.env.STELLAR_INSIGHTS_API_KEY ?? "smoke-test-dummy-key",
    STELLAR_INSIGHTS_MCP_ALLOW_WRITES: "true",
  },
});
const writeClient = new Client({ name: "smoke-test-write-client", version: "0.0.1" });
await writeClient.connect(writeTransport);
const writeTools = await writeClient.listTools();
console.log(`With ALLOW_WRITES=true: ${writeTools.tools.length} tools (was ${tools.tools.length} without it)`);
await writeClient.close();

process.exit(0);
