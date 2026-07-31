# @stellar-insights/mcp-server

An [MCP](https://modelcontextprotocol.io) server that exposes Stellar Insights
corridor, anchor, network and liquidity-pool analytics as tools for AI agents
(Claude, ChatGPT, and other MCP-compatible clients). It's a thin adapter over
[`@stellar-insights/sdk`](../typescript) — no new backend behaviour, just a
protocol translation layer — plus one tool (`verify_snapshot`) that reads
directly from the on-chain Soroban contract instead of going through the API.

Full design rationale (business model, architecture, work breakdown) lives in
[`docs/MCP_PLUGIN_PROPOSAL.md`](../../docs/MCP_PLUGIN_PROPOSAL.md).

## What's implemented (v0.1.0)

- 21 **read-only** tools covering corridors, anchors, prices, cost estimation,
  liquidity pools, network stats, ML anomaly prediction, governance proposals
  (read), alert history (read), and asset verification. See `src/tools.ts`
  for the full list — each maps 1:1 onto an existing SDK resource method.
- **`verify_snapshot`** — reads a snapshot hash directly from the deployed
  `stellar_insights` Soroban contract via a simulated (unsigned, read-only)
  transaction, with an optional `expected_hash` to compare against. This is
  the differentiator described in the proposal doc: the result can't have
  been tampered with off-chain, since the contract only accepts hashes via a
  `submit_snapshot` transaction. See `src/soroban.ts`. Verified live against
  the real deployed testnet contract (`CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL`)
  — see "Verifying your setup" below.
- **13 write-scoped tools**, gated behind `STELLAR_INSIGHTS_MCP_ALLOW_WRITES=true`
  (off by default): transaction submission, governance proposal creation/voting,
  alert rule CRUD, webhook CRUD, API key create/rotate/revoke. See
  `src/write-tools.ts`. Deliberately **not** included even when writes are
  enabled: anchor/corridor creation (not exposed by the SDK's resources at
  all) and `auth.login/refresh/logout` (the server already authenticates with
  one API key; there's no reason for an agent to manage a separate credential
  session).
- Two MCP resource templates (`stellar-insights://corridor/{source}/{destination}`,
  `stellar-insights://anchor/{id}`) so an agent can attach a specific corridor
  or anchor as context.
- **Two transports**: stdio (default, for Claude Desktop/Code) and streamable
  HTTP (`STELLAR_INSIGHTS_MCP_TRANSPORT=http`), stateless — a fresh server
  instance per request, no session state kept between calls. See
  `src/http-server.ts`.

## Known limitation

**No live end-to-end testing against the Stellar Insights REST backend.** The
backend does not currently compile from a clean clone (see the main repo
README / proposal doc's Phase 0 — tracked separately, not something this
package can fix). Everything backend-dependent here has been verified with
unit tests mocking `fetch` instead. What *has* been tested live, with no
mocking: `verify_snapshot` against the real deployed testnet Soroban contract,
the full stdio MCP handshake (tool/resource listing) with the real MCP
client/server libraries, and the HTTP transport via a raw `curl` MCP
`initialize` call.

## Setup

```bash
cd sdk/mcp-server
npm install
npm run build
```

## Configuration

| Env var | Required | Default | Purpose |
|---|---|---|---|
| `STELLAR_INSIGHTS_API_KEY` | yes | — | API key used to authenticate every REST-backed tool call (same key system as the REST API / other SDKs). Not needed for `verify_snapshot`, which only talks to Soroban RPC. |
| `STELLAR_INSIGHTS_NETWORK` | no | `testnet` | `mainnet` or `testnet` — selects the default API base URL, Soroban RPC URL, and network passphrase |
| `STELLAR_INSIGHTS_BASE_URL` | no | network default | Override the REST API base URL, e.g. to point at a local backend during development |
| `STELLAR_INSIGHTS_SOROBAN_RPC_URL` | no | network default | Override the Soroban RPC endpoint used by `verify_snapshot` |
| `STELLAR_INSIGHTS_SNAPSHOT_CONTRACT_ID` | only on mainnet | testnet: the deployed testnet contract ID | `stellar_insights` contract address for `verify_snapshot`. No mainnet deployment exists yet, so this must be set explicitly if you're running against mainnet. |
| `STELLAR_INSIGHTS_MCP_ALLOW_WRITES` | no | `false` | Enables the 13 write-scoped tools |
| `STELLAR_INSIGHTS_MCP_TRANSPORT` | no | `stdio` | Set to `http` to run the streamable-HTTP transport instead of stdio |
| `STELLAR_INSIGHTS_MCP_HTTP_PORT` | no | `3000` | Port for the HTTP transport (only used when `STELLAR_INSIGHTS_MCP_TRANSPORT=http`) |

## Running it

```bash
npm run build
STELLAR_INSIGHTS_API_KEY=your-key npm start
```

Or for local development without a build step:

```bash
STELLAR_INSIGHTS_API_KEY=your-key npm run dev
```

Over HTTP instead of stdio:

```bash
STELLAR_INSIGHTS_API_KEY=your-key STELLAR_INSIGHTS_MCP_TRANSPORT=http npm run dev
# POST MCP JSON-RPC requests to http://localhost:3000/mcp
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
                    # lists tools/resources, makes a LIVE verify_snapshot call
                    # against the real deployed testnet contract, and confirms
                    # STELLAR_INSIGHTS_MCP_ALLOW_WRITES adds the write tools
```
