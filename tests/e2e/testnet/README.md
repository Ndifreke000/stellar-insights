# E2E Testnet Payment Flow Tests

This directory contains end-to-end tests for the full payment flow on Stellar testnet.

## Overview

The `full_payment_flow` test validates the complete SEP-10 authentication and SEP-24 deposit workflow:

1. **Fund testnet account** — Calls Friendbot to fund a generated testnet keypair
2. **SEP-10 authentication** — Obtains a signed challenge from the backend and exchanges it for a JWT
3. **SEP-24 deposit initiation** — Uses the JWT to initiate an interactive deposit
4. **Dashboard verification** — Polls the analytics endpoint to confirm the transaction appears within 30 seconds

## Running Tests

### Prerequisites

- Rust toolchain (1.70+)
- Backend running locally or accessible via `BACKEND_URL`
- Network access to Stellar testnet services (Friendbot, anchor servers)

### Run the full test suite

```bash
TESTNET=true cargo test --test full_payment_flow -- --nocapture
```

### Run with custom backend URL

```bash
BACKEND_URL=https://api.example.com TESTNET=true cargo test --test full_payment_flow
```

## Environment Variables

| Variable | Default | Description |
| --- | --- | --- |
| `BACKEND_URL` | `http://localhost:8000` | Backend API base URL |
| `TESTNET` | (required) | Must be set to enable testnet tests |

## Expected Runtime

- **~30 seconds** — The test includes a 30-second polling window for dashboard appearance
- Faster in practice if transaction appears immediately

## ⚠️ Warning

**Do NOT run these tests against mainnet.** This test:

- Generates random keypairs and funds them on testnet
- Makes real requests to anchor services
- Is designed for Stellar Testnet only (`testnet.stellar.org`)

Modify `friendbot_url` and SEP-24 endpoints if using a different testnet-compatible network.

## Troubleshooting

| Issue | Solution |
| --- | --- |
| "Failed to call Friendbot" | Check network connectivity; Friendbot may be rate-limiting |
| "Challenge endpoint returned 401" | Verify `BACKEND_URL` is correct and backend is running |
| "Transaction did not appear within 30s" | Dashboard sync may be delayed; increase polling window or check transaction status manually |
