# PayRaider plugin for Claude

Check a Stellar payment corridor before paying out. The plugin gives Claude a
`preflight_payment` tool that returns a `proceed`, `caution`, `hold` or
`unknown` decision with the reasons behind it, a `preflight_batch` tool that
checks up to 100 payouts at once, plus read-only corridor, anchor, price and
asset-verification tools.

It is read-only. It never moves funds or signs anything.

## Install

```
/plugin marketplace add Ndifreke000/stellar-insights
/plugin install payraider@payraider
```

Node.js 20 or newer must be on your `PATH`. Nothing else is installed: the
server is a single bundled file, `server/server.mjs`.

## Configure

Set these in the environment Claude runs in:

| Variable | Required | Purpose |
| --- | --- | --- |
| `PAYRAIDER_BASE_URL` | yes, for your own backend | URL of the PayRaider backend to read from |
| `PAYRAIDER_NETWORK` | no | `mainnet` or `testnet` (default); only picks the default backend URL |
| `PAYRAIDER_API_KEY` | no | Without it the free anonymous tier applies |

## Use

- `/preflight USDC NGN 2500` runs the check directly.
- Or ask in plain language: "Is it safe to pay out 2,500 USD from USDC to NGN
  right now?" The `payment-preflight` skill tells Claude to run the check and
  how to report it.

## What is in here

| Path | Purpose |
| --- | --- |
| `.claude-plugin/plugin.json` | Plugin manifest |
| `.mcp.json` | Starts the bundled MCP server over stdio |
| `server/server.mjs` | The bundled server, built from [`sdk/mcp-server`](../../sdk/mcp-server); not committed |
| `skills/payment-preflight/` | When and how to run the check |
| `commands/preflight.md` | The `/preflight` command |

`server/server.mjs` is generated and gitignored on `main`, so the bundle never
shows up as an unreviewable diff. CI builds it from `sdk/mcp-server` and
publishes the whole plugin, bundle included, to the `plugin-dist` branch; the
marketplace installs from there. Each `plugin-dist` commit names the `main`
commit it was built from.

To run the plugin from a checkout, build the bundle first:

```bash
cd sdk/typescript && npm ci && npm run build
cd ../mcp-server && npm ci && npm run build:plugin
claude --plugin-dir ../../plugins/payraider
```

Hosting the server for several apps, and REST or SDK integration without
Claude, are covered in [`docs/PLUGIN.md`](../../docs/PLUGIN.md).
