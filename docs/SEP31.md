# SEP-31 (Cross-Border Payments) Integration

Stellar Insights integrates [SEP-31](https://developers.stellar.org/docs/platforms/anchor-platform/sep-guide/sep31) so users can send cross-border payments through anchors. Recipients can receive fiat; senders get quotes, initiate payments, and track status. KYC may be required by the anchor for sender or receiver.

## Overview

- **Get payment quotes** from anchors (SEP-38 style or anchor-specific).
- **Initiate cross-border payments** with amount, receiver, and optional quote.
- **Track payment status** and **view payment history**.
- **KYC**: Anchor may require sender/receiver customer (KYC) flows; use GET/PUT customer or anchor interactive flows.
- **Multiple corridors** (anchors) supported; optional JWT (SEP-10) for authenticated flows.

## Backend (Proxy API)

The backend proxies requests to anchor SEP-31 endpoints to avoid CORS and centralize allowed origins.

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/sep31/info?transfer_server=<url>` | Anchor capabilities (receive methods, quotes_supported). |
| POST | `/api/sep31/quote` | Get payment quote; body: `transfer_server`, `jwt`, `payload` (amount, sell_asset, buy_asset, etc.). |
| POST | `/api/sep31/transactions` | Create cross-border payment; body: `transfer_server`, `jwt`, `payload`. |
| GET | `/api/sep31/transactions?transfer_server=...&jwt=...&status=...&limit=...&cursor=...` | Payment history. |
| GET | `/api/sep31/transactions/:id?transfer_server=...&jwt=...` | Single payment details. |
| GET | `/api/sep31/customer?id=...&transfer_server=...&jwt=...` | KYC customer fetch. |
| PUT | `/api/sep31/customer` | KYC customer update (e.g. interactive callback). |
| GET | `/api/sep31/anchors` | List of configured SEP-31 anchors. |

### Configuration

- **`SEP31_ALLOWED_ORIGINS`** (optional): Comma-separated list of transfer server base URLs the proxy may call. If unset, any URL is allowed (dev only).
- **`SEP31_ANCHORS`** (optional): JSON array of preset anchors, e.g.:
  ```json
  [
    {
      "name": "Example Anchor",
      "transfer_server": "https://api.anchor.example/sep31",
      "home_domain": "anchor.example"
    }
  ]
  ```

### Error handling

- **403 Forbidden**: `transfer_server` not in `SEP31_ALLOWED_ORIGINS`.
- **502 Bad Gateway**: Proxy error (e.g. network failure to anchor).
- **4xx/5xx**: Forwarded from the anchor with the anchor’s response body.

## Frontend

- **Service**: `frontend/src/services/sep31.ts` – typed client for quotes, payments, transactions, customer, and anchors.
- **Component**: `frontend/src/components/Sep31PaymentFlow.tsx` – anchor selection, quote form, send payment form, payment history table, KYC note.
- **Page**: `frontend/src/app/send-payment/page.tsx` – Send payment page; linked from the sidebar.

### Flow

1. User selects a preset anchor or enters a SEP-31 transfer server URL.
2. App fetches `/api/sep31/info` for capabilities.
3. User enters amount, optional receiver ID, source/destination assets; optionally clicks **Get quote** (POST `/api/sep31/quote`).
4. User clicks **Send payment** (POST `/api/sep31/transactions`). Anchor may require KYC; complete any interactive flows the anchor provides.
5. Payment history is loaded via GET `/api/sep31/transactions` (JWT often required).

## Tests

- **Backend**: `backend/src/api/sep31_proxy.rs` – unit tests for `base_url`, quote body, and create-transaction body deserialization.
- Run: `cargo test -p stellar-insights-backend sep31_proxy`.

## Security notes

- Do not leave `SEP31_ALLOWED_ORIGINS` empty in production; restrict to trusted anchor SEP-31 base URLs.
- JWT (SEP-10) should be obtained and passed by the client; the proxy forwards it to the anchor.
