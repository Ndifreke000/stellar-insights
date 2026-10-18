// Boots the server over stdio, does a real MCP handshake and lists its tools
// and resource templates. No backend is contacted: listing does not call the
// PayRaider API, and no API key is needed.
//
//   node scripts/smoke-test.mjs            # the built dist/index.js
//   node scripts/smoke-test.mjs <file.mjs> # e.g. the plugin's bundled server
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";

const packageRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const entry = resolve(process.argv[2] ?? join(packageRoot, "dist", "index.js"));

const transport = new StdioClientTransport({
  command: process.execPath,
  args: [entry, "--stdio"],
  env: { ...process.env, PAYRAIDER_API_KEY: "" },
});

const client = new Client({ name: "smoke-test-client", version: "0.0.1" });
await client.connect(transport);

const { tools } = await client.listTools();
const { resourceTemplates } = await client.listResourceTemplates();
await client.close();

console.log(`${entry}`);
console.log(`  ${tools.length} tools: ${tools.map((t) => t.name).join(", ")}`);
console.log(`  resources: ${resourceTemplates.map((r) => r.uriTemplate).join(", ")}`);

if (tools[0]?.name !== "preflight_payment" || resourceTemplates.length !== 2) {
  console.error("unexpected tool or resource listing");
  process.exit(1);
}
