# @stellar-insights/mcp-server

An [MCP](https://modelcontextprotocol.io) server that exposes Stellar Insights
corridor, anchor, network and liquidity-pool analytics as tools for AI agents
(Claude, ChatGPT, and other MCP-compatible clients). It's a thin adapter over
[`@stellar-insights/sdk`](../typescript) — no new backend behaviour, just a
protocol translation layer.

Full design rationale (business model, architecture, work breakdown) lives in
[`docs/MCP_PLUGIN_PROPOSAL.md`](../../docs/MCP_PLUGIN_PROPOSAL.md).

## What's implemented (v0.1.0)

- 21 **read-only** tools covering corridors, anchors, prices, cost estimation,
  liquidity pools, network stats, ML anomaly prediction, governance proposals
  (read), alert history (read), and asset verification. See `src/tools.ts`
  for the full list — each maps 1:1 onto an existing SDK resource method.
- Two MCP resource templates (`stellar-insights://corridor/{source}/{destination}`,
  `stellar-insights://anchor/{id}`) so an agent can attach a specific corridor
  or anchor as context.
- stdio transport (the standard local-process MCP transport).

## Not yet implemented

- `verify_snapshot` (on-chain Soroban snapshot hash verification) — the
  differentiator tool described in the proposal doc. Needs a Soroban RPC
  contract-read integration that hasn't been built yet.
- Write-scoped tools (transactions, governance votes, alert-rule/webhook
  management). `STELLAR_INSIGHTS_MCP_ALLOW_WRITES=true` is reserved for this
  but is currently a no-op — see `src/index.ts`.
- Streamable-HTTP transport (stdio only for now).
- **Live end-to-end testing against a real backend.** The backend does not
  currently compile from a clean clone (redis-rs API mismatch, see the main
  repo README / proposal doc's Phase 0). Everything in this package has been
  verified with: unit tests mocking `fetch` (`npm test`), a real MCP
  handshake smoke test that boots the server and lists its tools/resources
  over stdio (`npm run smoke-test`), and `tsc --noEmit`. None of that has hit
  a live backend yet.

## Setup

```bash
cd sdk/mcp-server
npm install
npm run build
```

## Configuration

| Env var | Required | Default | Purpose |
|---|---|---|---|
| `STELLAR_INSIGHTS_API_KEY` | yes | — | API key used to authenticate every tool call (same key system as the REST API / other SDKs) |
| `STELLAR_INSIGHTS_NETWORK` | no | `testnet` | `mainnet` or `testnet` — selects the default API base URL |
| `STELLAR_INSIGHTS_BASE_URL` | no | network default | Override, e.g. to point at a local backend during development |
| `STELLAR_INSIGHTS_MCP_ALLOW_WRITES` | no | `false` | Reserved for write-scoped tools (not yet implemented) |

## Running it

```bash
npm run build
STELLAR_INSIGHTS_API_KEY=your-key npm start
```

Or for local development without a build step:

```bash
STELLAR_INSIGHTS_API_KEY=your-key npm run dev
```

### Claude Desktop / Claude Code config

```json
{
  "mcpServers": {
    "stellar-insights": {
      "command": "node",
      "args": ["/absolute/path/to/sdk/mcp-server/dist/index.js"],
      "env": { "STELLAR_INSIGHTS_API_KEY": "your-key" }
    }
  }
}
```

## Verifying your setup

```bash
npm test          # unit tests, mocked HTTP, no network required
npm run smoke-test # boots the real server, does a real MCP handshake over stdio,
                    # lists its tools/resources (uses a dummy API key - no live
                    # backend call is made by tools/list itself)
```
