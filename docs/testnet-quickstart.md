# Testnet Quickstart Guide

This guide describes how to connect the entire Stellar Insights stack (backend, frontend, mobile, contracts, and SDK examples) to the Stellar Testnet.

> Verified against a real, start-to-finish testnet deployment (fresh keypair → Friendbot funding → all 9 contracts deployed via `scripts/deploy-contracts-testnet.sh`, IDs recorded in [`contracts/.env.testnet`](../contracts/.env.testnet)). The steps and gotchas below reflect what actually happened, not just what was expected to work.

## Prerequisites

1. **Rust with the `wasm32v1-none` target.**
   Soroban contracts in this workspace build against `soroban-sdk = "26.0.1"`, which targets `wasm32v1-none` — **not** the `wasm32-unknown-unknown` target that most Rust installs default to for WASM. Building without installing this target first fails. Install it explicitly:
   ```bash
   rustup target add wasm32v1-none
   ```
   (CI does the same — see [`.github/workflows/contract-fuzzing.yml`](../.github/workflows/contract-fuzzing.yml), which passes `targets: wasm32v1-none` to the toolchain action.)

2. **Stellar CLI.**
   ```bash
   cargo install --locked stellar-cli
   ```
   If `cargo install` can't reach crates.io (blocked registries, restricted sandboxes, etc.), install a prebuilt binary from the [stellar-cli GitHub releases](https://github.com/stellar/stellar-cli/releases/latest) instead — this is a documented working fallback. For example, on Linux x86_64:
   ```bash
   VERSION=27.0.0   # check the releases page for the current version
   curl -LO "https://github.com/stellar/stellar-cli/releases/download/v${VERSION}/stellar-cli-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
   tar -xzf "stellar-cli-${VERSION}-x86_64-unknown-linux-gnu.tar.gz"
   sudo mv stellar /usr/local/bin/
   ```
   macOS (`aarch64-apple-darwin` / `x86_64-apple-darwin`), Windows (`x86_64-pc-windows-msvc`, `.tar.gz` or an `.exe` installer), and `.deb` packages are published under the same release.
   Verify with `stellar --version`.

---

## Fund a testnet account with Friendbot

Before deploying contracts or making transactions, you need a funded Stellar account on the testnet.

1. **Generate a new keypair**:
   Using the Stellar CLI:
   ```bash
   stellar keys generate --network testnet my-identity
   ```
   Or using standard tools to obtain a Public Key (starting with `G`) and a Secret Key (starting with `S`).

2. **Fund the account with Friendbot**:
   You can fund your new account via the Stellar CLI:
   ```bash
   stellar keys fund my-identity --network testnet
   ```
   Alternatively, trigger Friendbot using `curl`:
   ```bash
   curl "https://friendbot.stellar.org/?addr=YOUR_STELLAR_PUBLIC_KEY"
   ```

---

## Configure backend .env for testnet

To configure the backend to read from the testnet:

1. Copy `backend/.env.example` to `backend/.env`.
2. Update the network environment variables:
   ```ini
   # Set network to testnet
   STELLAR_NETWORK=testnet
   STELLAR_NETWORK_PASSPHRASE="Test SDF Network ; September 2015"

   # SEP-10 server key (must start with G and be 56 characters)
   SEP10_SERVER_PUBLIC_KEY=YOUR_TESTNET_PUBLIC_KEY
   SEP10_HOME_DOMAIN=localhost:8080
   ```
3. Verify that the testnet RPC/Horizon endpoints are set correctly:
   ```ini
   STELLAR_RPC_URL_TESTNET=https://soroban-testnet.stellar.org
   STELLAR_HORIZON_URL_TESTNET=https://horizon-testnet.stellar.org
   ```

---

## Deploy contracts to testnet

The workspace has **9 contracts** with dependency ordering (`access-control` and `stellar_insights` are depended on by most of the others). Deploying them one at a time by hand is error-prone — use the deploy script, which builds everything, deploys in the correct order, and records the results.

1. **Make sure prerequisites are installed** (see above): `rustup target add wasm32v1-none` and the Stellar CLI.

2. **Run the deploy script** from the repo root:
   ```bash
   ./scripts/deploy-contracts-testnet.sh --source my-identity
   ```
   (`my-identity` is the identity created in [Fund a testnet account](#fund-a-testnet-account-with-friendbot) above. You can also set `STELLAR_ACCOUNT=my-identity` instead of passing `--source`, and override the transaction fee with `--fee <stroops>`, default `100`.)

   This builds all contracts for release (`cargo build --release --target wasm32v1-none`), then deploys them in dependency order:
   `access-control → stellar_insights → analytics → governance → escrow → token-swap → multi-sig-wallet → time-locked-transactions → upgrade`.

3. **Contract IDs are written to `contracts/.env.testnet`** as the deploy progresses (one line appended per successful deploy, so a partial run still leaves you with the IDs deployed so far). Source it to use the IDs in your shell:
   ```bash
   source contracts/.env.testnet
   ```

   This repo's `contracts/.env.testnet` already has the IDs from a real deployment, committed for reference:

   | Contract | Env var | Contract ID |
   |---|---|---|
   | access-control | `ACCESS_CONTROL_CONTRACT_ID` | `CAZO4LD7NSWZFUJCB5ORHS3IBFJC76KHSOCPTHVDBDBISZJ72ACSHPH5` |
   | stellar_insights | `STELLAR_INSIGHTS_CONTRACT_ID` | `CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL` |
   | analytics | `ANALYTICS_CONTRACT_ID` | `CAJAIBW6BXRSM4CW76KRWM2UVDJOWYYPJROO2EIHB376MP6V3PGTORNF` |
   | governance | `GOVERNANCE_CONTRACT_ID` | `CCBGEJY2CNM7XOMV5D25NARRW6MKMFW3XOU72YQOMC7VUWHNENHM3JQV` |
   | escrow | `ESCROW_CONTRACT_ID` | `CCI3YVIWBXM5YMOJZGN26CZDVQ2WRZJFSOLU6FCF4KDZ3O2KFCI7RSI2` |
   | token-swap | `TOKEN_SWAP_CONTRACT_ID` | `CDUQO3ZZICNSRIUHGA6EAL2KIMKDYFURBVGOPWDEJKPJ3P2WKSHPIOQN` |
   | multi-sig-wallet | `MULTI_SIG_WALLET_CONTRACT_ID` | `CC6I665SUO7IY5IEQ4YGPBHTIX6HJJ5GB3BUQQFBB2KUI7GQT2AITLPK` |
   | time-locked-transactions | `TIME_LOCKED_TRANSACTIONS_CONTRACT_ID` | `CBAFVZVCQJQXXK5BBKIX7ERC7ULYNTGF3DFLTFSF4GYE77GNUOOOYIJV` |
   | upgrade | `UPGRADE_CONTRACT_ID` | `CBT3WW736ZU3GYPCE5S6GB4QHX3FDFSC53LZRSBRHQBMRD2HWR7DXZXB` |

   These are real, already-deployed IDs — if you just need *a* testnet deployment to point the backend/frontend at, you don't have to redeploy; use `contracts/.env.testnet` directly. Only re-run the script if you've changed contract code and need fresh IDs (there's no upgrade-in-place for a new WASM hash on a new contract ID — each deploy is a new contract).

### Deploying a single contract manually

If you only need to (re)deploy one contract, e.g. after a local code change:
```bash
cd contracts
cargo build --target wasm32v1-none --release
stellar contract deploy \
  --wasm target/wasm32v1-none/release/stellar_insights.wasm \
  --source my-identity \
  --network testnet
```
The command prints the new Contract ID on success — a 56-character StrKey string starting with `C` (not a hex string), e.g. `CAPHQZ4BBT43HU5EUSJAOPKWB66HGLTN4AKJUALV3R2RXS4A6IOXWUTL`.

---

## Run frontend against testnet API

To point the Next.js frontend to the testnet backend API:

1. Create a `frontend/.env.local` file.
2. Set the backend API URL:
   ```ini
   NEXT_PUBLIC_API_URL=http://localhost:8080/api/v1
   NEXT_PUBLIC_WS_URL=ws://localhost:8080/ws
   ```
3. Run the development server:
   ```bash
   cd frontend
   npm run dev
   ```

---

## Run mobile app against testnet

To run the React Native mobile app pointing to the testnet:

1. Copy `mobile/.env.example` to `mobile/.env`.
2. Configure the backend API URL and the Stellar network settings:
   ```ini
   # API Configuration pointing to local backend (or deployed staging backend)
   # For Android emulator, use 10.0.2.2 instead of localhost
   API_BASE_URL=http://localhost:8080/api/v1

   # Stellar Network Configuration
   STELLAR_NETWORK=testnet
   STELLAR_HORIZON_URL=https://horizon-testnet.stellar.org
   ```
3. Run the mobile application:
   ```bash
   cd mobile
   npm run android  # or npm run ios
   ```

---

## Run SDK examples against testnet

To run SDK examples pointing to the testnet:

1. Verify that your SDK configuration imports the testnet passphrase and endpoint.
2. Example script usage:
   ```javascript
   import { StellarSdk } from '@stellar-insights/sdk';

   const sdk = new StellarSdk({
     network: 'testnet',
     rpcUrl: 'https://soroban-testnet.stellar.org',
     horizonUrl: 'https://horizon-testnet.stellar.org'
   });

   // Run your integration script
   await sdk.fetchCorridors();
   ```

---

## Continuous integration

[`.github/workflows/deploy-testnet.yml`](../.github/workflows/deploy-testnet.yml) runs on every push to `main` touching `backend/**`, `k8s/**`, or the workflow file itself (and via manual `workflow_dispatch`). Its test step runs:
```bash
cargo test --features sep-integration
```
**Not** `--all-features`. `legacy_sep10_tests` gates `backend/tests/sep10_test.rs`, which targets a pre-consolidation crate layout (`stellar_base`/`stellar_analytics`) that no longer exists in this repo and hasn't compiled for some time — `--all-features` will fail the build. If you're running backend tests locally against testnet config, match CI and use `--features sep-integration` rather than `--all-features`.

Note this pipeline only builds/deploys the **backend service** (Docker image → the `k8s/overlays/testnet` Kustomize overlay, applied via `kubectl`); it does not touch the Soroban contracts. Contract deployment is the separate, manual `scripts/deploy-contracts-testnet.sh` flow described above — there's no CI job that redeploys contracts automatically.
