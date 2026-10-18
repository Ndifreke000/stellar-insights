# PayRaider

> ⚠️ **DEPRECATED — Development has moved to the [Pay-Raider organisation](https://github.com/Pay-Raider).**
>
> This repository is **archived and no longer maintained**. No new features, bug fixes, or security patches will be merged here. All active development continues in the Pay-Raider org:
>
> | Component | New home |
> | --- | --- |
> | Backend API (Rust) | [Pay-Raider/payraider-backend](https://github.com/Pay-Raider/payraider-backend) |
> | Web app (Next.js) | [Pay-Raider/payraider-app](https://github.com/Pay-Raider/payraider-app) |
> | Plugin & SDKs | [Pay-Raider/payraider-plugin](https://github.com/Pay-Raider/payraider-plugin) |
> | Soroban contracts | [Pay-Raider/payraider-contracts](https://github.com/Pay-Raider/payraider-contracts) |
> | Mobile app | [Pay-Raider/payraider-mobile](https://github.com/Pay-Raider/payraider-mobile) |
>
> **If you have open issues or PRs here**, please re-open them in the relevant Pay-Raider repository. See the [migration checklist](#migration-checklist--archive-plan) below.

## Migration Checklist & Archive Plan

### What has migrated

- [x] Core preflight API (`/api/v1/preflight`) → `payraider-backend`
- [x] TypeScript and Python SDKs → `payraider-plugin`
- [x] MCP server → `payraider-plugin`
- [x] Claude plugin → `payraider-plugin`
- [x] Soroban snapshot contract → `payraider-contracts`
- [x] Next.js dashboard and corridor explorer → `payraider-app`
- [x] React Native mobile app → `payraider-mobile`

### What stays here (history only)

- Earlier-scope features: governance tooling, quest system, GDPR tooling — not carried forward.
- All git history, tags, and releases are preserved in this repo for reference.

### Archive plan

1. All open issues labelled `migrate` will be recreated in the appropriate Pay-Raider repo.
2. This repo will be set to **read-only / archived** on GitHub once the issue backlog is triaged (target: next sprint after this PR merges).
3. A redirect banner (this notice) will remain in the README permanently.
4. The `main` branch will receive no further commits after archiving.

[![Coverage](https://codecov.io/gh/Ndifreke000/stellar-insights/branch/main/graph/badge.svg)](https://codecov.io/gh/Ndifreke000/stellar-insights)
[![Coverage CI](https://github.com/Ndifreke000/stellar-insights/actions/workflows/coverage.yml/badge.svg)](https://github.com/Ndifreke000/stellar-insights/actions/workflows/coverage.yml)

**Check a Stellar payment corridor before you pay out.**

Off-ramps and payout apps lose money when a payment goes out on a corridor
that is degraded or too thin for the amount. PayRaider answers one question
before the money moves: *is this corridor healthy enough to pay on right
now?*

```bash
curl -s "https://<your-payraider-host>/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

returns `proceed`, `caution`, `hold` or `unknown`, the checks behind it
(success rate of recent payments including failed ones, liquidity against the
amount, sample size, corridor health) and healthier corridors to the same
currency.

- **Observed, not modelled.** Decisions come from recent payments on the
  Stellar ledger. No data means `unknown`, never a guess.
- **Free to try.** No API key needed; keys raise the rate limit.
- **Paid in USDC on Stellar.** The Pro plan is bought with a USDC payment that
  the API verifies on the ledger.
- **Fits any stack.** REST, TypeScript and Python SDKs, an MCP server for AI
  agents, and a Claude plugin.

Integration guide: [docs/PLUGIN.md](docs/PLUGIN.md).

### What is in this repository

| Path | What it is |
| --- | --- |
| `backend/` | Rust API (Axum, SQLite): corridor data, the pre-payment check, wallet sign-in, API keys and USDC billing |
| `sdk/typescript`, `sdk/python`, `sdk/react` | Client SDKs and React hooks — releases: [sdk/RELEASING.md](sdk/RELEASING.md), versions: [sdk/COMPATIBILITY.md](sdk/COMPATIBILITY.md) |
| `sdk/mcp-server`, `plugins/payraider` | MCP server and Claude plugin |
| `frontend/` | Next.js dashboard, corridor explorer and self-serve API keys |
| `contracts/` | The `payraider` Soroban contract that anchors snapshot hashes on-chain — how to verify a snapshot: [docs/SNAPSHOT_ANCHORING.md](docs/SNAPSHOT_ANCHORING.md) |
| `mobile/` | React Native app ([status](mobile/README.md#current-status)) |

The dashboard also carries features from the project's earlier scope
(governance, quests, GDPR tooling and others). They are kept but are not part
of the off-ramp product.

## 🔧 Quick start

No database server to install — SQLite runs in-process and the file is created
on first start.

1. Run backend

```bash
cd backend
cp .env.example .env
# DATABASE_URL already defaults to sqlite:./payraider.db
# replace the CHANGE_ME placeholders: the backend refuses to start with them
#   JWT_SECRET=$(openssl rand -base64 48)
#   ENCRYPTION_KEY=$(openssl rand -hex 32)
#   SEP10_SERVER_PUBLIC_KEY=<a real Stellar public key>
# set STELLAR_RPC_URL etc. as needed
cargo run
```

2. Run frontend

```bash
cd frontend
cp .env.example .env.local   # NEXT_PUBLIC_API_URL must point at the backend
pnpm install
pnpm dev
```

## Plugin: check a corridor before you pay

Off-ramp and payout apps can ask whether a corridor is healthy enough to pay
on right now. The check is public and read-only, and needs no API key:

```bash
curl -s -X POST http://localhost:8080/api/v1/preflight \
  -H 'content-type: application/json' \
  -d '{"source_asset":"USDC","destination_asset":"NGN","amount_usd":2500}'
```

It returns `proceed`, `caution`, `hold` or `unknown` with the checks behind the
decision and healthier alternative corridors. The same check is available
through the TypeScript and Python SDKs, as an MCP server you can host
(`sdk/mcp-server`), and as a Claude plugin (`plugins/payraider`). See
[docs/PLUGIN.md](docs/PLUGIN.md).

### Corridors are keyed by asset code **and** issuer

A corridor is identified by the full asset, not just its code. `USDC` issued
by Circle and `USDC` issued by another anchor are **different corridors** with
different liquidity and risk, so they are tracked and evaluated separately.

Pass the issuer with `CODE:ISSUER` to pin a specific asset:

```bash
curl -s -X POST http://localhost:8080/api/v1/preflight \
  -H 'content-type: application/json' \
  -d '{"source_asset":"USDC:GA5ZSEJYB37JRC5AVCIA5MOP4RHTM335X2KGX3IHOJAPP5RE34K4KZVN","destination_asset":"NGN","amount_usd":2500}'
```

A bare `CODE` (e.g. `USDC`) is still accepted and resolves to the corridor for
that code; when several issuers exist for the same code, use `CODE:ISSUER` to
select the one you mean. Malformed asset strings are rejected at the boundary
with the standard error response.

## Database

**SQLite only.** This is a compile-time property, not a configuration one:
`sqlx` is built with only the `sqlite` feature, `backend/src/database.rs` uses
`SqlitePool`/`SqliteConnectOptions` directly, and every migration is written in
SQLite-flavoured SQL.

Setting a `postgresql://` `DATABASE_URL` does **not** switch databases — it
fails to parse as `SqliteConnectOptions` and the backend refuses to start.
Adding Postgres support is a code change. Whether to make it is being tracked
in issue #1876.

## Deployed contracts

The backend integrates with **`payraider`** via `SNAPSHOT_CONTRACT_ID`.
[`contracts/.env.testnet`](contracts/.env.testnet) is generated by
`deploy-contracts-testnet.sh` — prefer reading it over copying IDs from here.

| Contract | Env var | Testnet ID (historical) |
|---|---|---|
| PayRaider | `PAYRAIDER_CONTRACT_ID` / `SNAPSHOT_CONTRACT_ID` | `CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL` |

All other contract crates live under `contracts/archive/`, are excluded from the
Cargo workspace and are not built in CI.
See [`contracts/archive/README.md`](contracts/archive/README.md).
Who may anchor snapshots, and how to rotate those keys, is documented in
[`contracts/payraider/README.md`](contracts/payraider/README.md).

## 🧪 Local safety checks (already built)

- `scripts/check_folder_size.sh` (fails if any folder >200MB)
- [`.github/workflows/enforce-folder-size.yml`](.github/workflows/enforce-folder-size.yml)

## 📁 Structure

| Path | Contents |
|---|---|
| `backend/` | Rust analytics engine — REST, GraphQL and WebSocket API |
| `frontend/` | Next.js dashboard |
| `mobile/` | React Native (Expo) app — [current status](mobile/README.md#current-status) |
| `contracts/` | Soroban contracts (Rust, `no_std`) |
| `sdk/` | TypeScript and Python API clients |
| `docs/` | Project documentation |
| `scripts/` | Build, deploy and maintenance scripts |
| `tests/` | Cross-cutting / end-to-end tests |
| `k8s/`, `terraform/` | Deployment manifests and infrastructure as code — **`terraform/` is not used by the actual deployment** (Render Blueprint + SQLite, see `docs/DEPLOY.md`); kept as a reference, see issue #2650 |
| `elk/`, `monitoring/` | Logging and observability stack configs — **`elk/` is not used by the actual deployment**; kept as a reference, see issue #2650 |

## 📌 Notes

- History has been cleaned to reduce clone size.
- Use `git lfs` for large binaries.
- For full operational details, see `docs/` and module READMEs.

## SDK status

`sdk/` ships four packages: `typescript`, `python

/* … truncated 1302 chars — edit only what you need near the top … */

## Handsoff notes

<!-- handsoff-issue-2608 -->
- #2608: Data retention and downsampling for raw payment observations
