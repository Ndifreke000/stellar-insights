# SEP-24 (Hosted Deposit & Withdrawal) Integration

Stellar Insights integrates [SEP-24](https://developers.stellar.org/docs/platforms/anchor-platform/sep-guide/sep24) so users can deposit and withdraw fiat (and assets) through anchors directly from the platform, with a single UI for anchor interactions.

## Overview

- **Discover** SEP-24–enabled anchors (from config or custom transfer server URL).
- **Deposit** and **Withdraw** via interactive flows (KYC, etc.) in a popup.
- **Track** transaction status and **view** transaction history.
- **Multiple anchors** supported; optional JWT (SEP-10) for authenticated flows.

## Backend (Proxy API)

The backend proxies requests to anchor transfer servers to avoid CORS and to centralize allowed origins.

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/sep24/info?transfer_server=<url>` | Anchor capabilities (deposit/withdraw assets and limits). |
| POST | `/api/sep24/deposit/interactive` | Start interactive deposit; returns URL for popup. |
| POST | `/api/sep24/withdraw/interactive` | Start interactive withdrawal; returns URL for popup. |
| GET | `/api/sep24/transactions?transfer_server=...&jwt=...&...` | Transaction history. |
| GET | `/api/sep24/transaction?transfer_server=...&id=...&jwt=...` | Single transaction details. |
| GET | `/api/sep24/anchors` | List of configured SEP-24 anchors. |

### Configuration

- **`SEP24_ALLOWED_ORIGINS`** (optional): Comma-separated list of transfer server base URLs that the proxy may call. If unset, any URL is allowed (suitable only for development).
- **`SEP24_ANCHORS`** (optional): JSON array of preset anchors for discovery, e.g.:
  ```json
  [
    {
      "name": "Example Anchor",
      "transfer_server": "https://api.anchor.example/sep24",
      "home_domain": "anchor.example"
    }
  ]
  ```

### Error handling

- **403 Forbidden**: `transfer_server` not in `SEP24_ALLOWED_ORIGINS`.
- **502 Bad Gateway**: Proxy error (e.g. network failure talking to the anchor).
- **4xx/5xx**: Forwarded from the anchor with the anchor’s response body.

## Frontend

- **Service**: `frontend/src/services/sep24.ts` – typed client for all proxy endpoints and error handling.
- **Component**: `frontend/src/components/Sep24Flow.tsx` – anchor selection, deposit/withdraw form, interactive URL handling, transaction history table.
- **Page**: `frontend/src/app/deposit-withdraw/page.tsx` – Deposit & Withdraw page; linked from the sidebar.

### Flow

1. User selects a preset anchor or enters a transfer server URL.
2. App fetches `/api/sep24/info` and shows deposit/withdraw assets and form.
3. User picks Deposit or Withdraw, optional asset, amount, account, JWT.
4. App calls `/api/sep24/deposit/interactive` or `/api/sep24/withdraw/interactive` and opens the returned URL in a popup for KYC/interactive flow.
5. Transaction history is loaded via `/api/sep24/transactions` (JWT optional but required by many anchors).

## Tests

- **Backend**: `backend/src/api/sep24_proxy.rs` – unit tests for `base_url`, request body deserialization.
- Run: `cargo test -p stellar-insights-backend sep24_proxy`.

## Security notes

- Do not leave `SEP24_ALLOWED_ORIGINS` empty in production; restrict to trusted anchor transfer server URLs.
- JWT (SEP-10) should be obtained and passed by the client; the proxy forwards it to the anchor.
