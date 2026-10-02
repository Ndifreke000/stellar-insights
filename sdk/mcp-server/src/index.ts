#!/usr/bin/env node
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { ConfigError, loadConfig } from "./config.js";
import { listen } from "./http.js";
import { buildServer } from "./server.js";
import { VERSION } from "./version.js";

const HELP = `payraider-mcp-server ${VERSION}

Exposes PayRaider payment-corridor analytics and the pre-payment check as MCP
tools.

Usage:
  payraider-mcp-server [--stdio | --http]

Options:
  --stdio      Speak MCP over stdin/stdout (default; for local clients)
  --http       Serve MCP over streamable HTTP at POST /mcp (for hosting)
  --version    Print the version
  --help       Print this help

Environment:
  PAYRAIDER_BASE_URL             Backend URL (default: the network's hosted API)
  PAYRAIDER_NETWORK              mainnet | testnet (default: testnet)
  PAYRAIDER_API_KEY              Optional; without it the free tier applies
  PAYRAIDER_MCP_TRANSPORT        stdio | http (default: stdio)
  HOST, PORT                     HTTP bind address (default: 127.0.0.1:3333)
  PAYRAIDER_MCP_AUTH_TOKEN       Bearer token required on the HTTP endpoint
  PAYRAIDER_MCP_ALLOWED_ORIGINS  Comma-separated browser origins to allow
`;

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  if (argv.includes("--help") || argv.includes("-h")) {
    process.stdout.write(HELP);
    return;
  }
  if (argv.includes("--version") || argv.includes("-v")) {
    process.stdout.write(`${VERSION}\n`);
    return;
  }

  const config = loadConfig(process.env, argv);

  if (config.transport === "http") {
    const server = await listen(config);
    // stdout is reserved for the protocol in stdio mode; log to stderr in both.
    console.error(
      `payraider-mcp-server ${VERSION} listening on http://${config.host}:${config.port}/mcp ` +
        `(backend ${config.baseUrl}, ${config.apiKey ? "API key" : "free tier"}` +
        `${config.authToken ? ", bearer auth on" : ""})`,
    );
    const shutdown = () => server.close(() => process.exit(0));
    process.on("SIGTERM", shutdown);
    process.on("SIGINT", shutdown);
    return;
  }

  await buildServer(config).connect(new StdioServerTransport());
}

main().catch((err) => {
  if (err instanceof ConfigError) {
    console.error(`payraider-mcp-server: ${err.message}`);
  } else {
    console.error("payraider-mcp-server failed to start:", err);
  }
  process.exit(1);
});
