# @payraider/mcp-server

An [MCP](https://modelcontextprotocol.io) server that lets an off-ramp or
payout app, or an AI agent working for one, **check a Stellar payment corridor
before paying out**, and read the corridor, anchor, price and
asset-verification data behind that check.

It is a thin adapter over [`@payraider/sdk`](../typescript). It is read-only:
it never moves funds or signs anything.

For REST and SDK integration without MCP, and for installing it as a Claude
plugin, see [`docs/PLUGIN.md`](../../docs/PLUGIN.md).

## The pre-payment check

`preflight_payment` takes a source asset, a destination asset and an amount in
USD, and returns one decision:

| Decision | Meaning |
| --- | --- |
| `proceed` | Every check passed |
| `caution` | At least one check is marginal; pay with care or use an alternative |
| `hold` | At least one check failed; do not pay on this corridor now |
| `unknown` | No recent payments on this corridor, so no recommendation |

with the checks that produced it (success rate, liquidity headroom for the
amount, p95 latency, sample size, health score) and healthier alternative
corridors to the same destination asset.

It works **without an API key**. The backend's anonymous rate-limit tier
applies; set `PAYRAIDER_API_KEY` for a higher limit.

## Tools

| Tool | What it does |
| --- | --- |
| `preflight_payment` | Proceed / caution / hold decision for a corridor and amount |
| `list_corridors`, `get_corridor` | Corridor health, latency, liquidity and history |
| `list_anchors`, `get_anchor`, `get_anchor_by_account` | Anchor directory and reliability |
| `get_price`, `get_prices`, `convert_to_usd` | USD prices and conversion |
| `estimate_transfer_cost` | Fees, spread and slippage across payment routes |
| `list_liquidity_pools`, `get_liquidity_pool` | AMM liquidity pools |
| `get_network_info`, `list_available_networks` | Which Stellar network is reported on |
| `list_governance_proposals`, `get_governance_proposal` | Governance proposals (read) |
| `verify_asset`, `get_verified_asset`, `list_verified_assets` | Asset verification |
| `list_alert_history` | Past alerts; needs a signed-in user token, not just an API key |

Two resource templates let a client attach an entity as context:
`payraider://corridor/{source}/{destination}` and `payraider://anchor/{id}`.

## Run it

Requires Node.js 20 or newer. The SDK is linked from this repository, so build
it first:

```bash
cd sdk/typescript && npm ci && npm run build
cd ../mcp-server  && npm ci && npm run build
```

### Local client (stdio)

```bash
PAYRAIDER_BASE_URL=http://localhost:8080 npm start
```

Claude Desktop / Claude Code config:

```json
{
  "mcpServers": {
    "payraider": {
      "command": "node",
      "args": ["/absolute/path/to/sdk/mcp-server/dist/index.js"],
      "env": { "PAYRAIDER_BASE_URL": "http://localhost:8080" }
    }
  }
}
```

### Hosted (streamable HTTP)

```bash
PAYRAIDER_BASE_URL=https://your-backend.example.com \
PAYRAIDER_MCP_AUTH_TOKEN=$(openssl rand -hex 32) \
HOST=0.0.0.0 PORT=3333 npm start -- --http
```

- `POST /mcp` is the MCP endpoint. It is stateless, so instances can sit behind
  a load balancer with no session affinity.
- `GET /healthz` is a liveness probe.

### Docker

```bash
# from the sdk/ directory
docker build -f mcp-server/Dockerfile -t payraider-mcp .
docker run --rm -p 3333:3333 \
  -e PAYRAIDER_BASE_URL=https://your-backend.example.com \
  -e PAYRAIDER_MCP_AUTH_TOKEN=change-me \
  payraider-mcp
```

Or `cp .env.example .env`, fill it in, and `docker compose up --build`. The
image contains a single bundled file, runs as an unprivileged user and has a
health check. A [Fly.io config](fly.toml) is included; it has not been deployed
from this repository.

## Configuration

| Variable | Default | Purpose |
| --- | --- | --- |
| `PAYRAIDER_BASE_URL` | the network's hosted API | Backend the server reads from. Set this to your own deployment. |
| `PAYRAIDER_NETWORK` | `testnet` | `mainnet` or `testnet`; only picks the default base URL |
| `PAYRAIDER_API_KEY` | none | Optional. Without it the free anonymous tier applies |
| `PAYRAIDER_MCP_TRANSPORT` | `stdio` | `stdio` or `http` (`--stdio` / `--http` override it) |
| `HOST`, `PORT` | `127.0.0.1`, `3333` | HTTP bind address |
| `PAYRAIDER_MCP_AUTH_TOKEN` | none | Bearer token required on `/mcp` |
| `PAYRAIDER_MCP_ALLOWED_ORIGINS` | none | Comma-separated browser origins allowed to call `/mcp` |

## Security

- **Read-only.** There are no write tools.
- **Set `PAYRAIDER_MCP_AUTH_TOKEN` on any deployment reachable from the
  internet.** Without it anyone who can reach `/mcp` can spend your API key's
  quota.
- Requests carrying an `Origin` header are rejected unless that origin is in
  `PAYRAIDER_MCP_ALLOWED_ORIGINS`, which stops a web page from driving a
  locally running server.
- The HTTP server binds to loopback by default; the Docker image binds to
  `0.0.0.0`.

## Develop

```bash
npm run lint        # type-check src and tests
npm test            # unit and integration tests against a local fake backend
npm run smoke-test  # real MCP handshake over stdio, no backend needed
npm run build:plugin  # rebuild the bundle shipped in plugins/payraider
```

## Limits

- Corridor metrics come from recently observed payments. A corridor with no
  recent payments returns `unknown` rather than a guess.
- The backend records only successful payments from the Stellar payment
  stream, so the success-rate check reflects that data source.
- Price tools depend on the backend's upstream price feed being reachable.
- `get_anchor` expects the anchor ID as stored by the backend.
- There is no ML prediction tool: the backend does not serve one.
