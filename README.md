# stellar-insights

Stellar Insights provides preflight checks and analytics for Stellar payment corridors.

## Getting started

```bash
git clone https://github.com/Ndifreke000/stellar-insights.git
cd stellar-insights
# backend (Rust, SQLite created on first start)
cd backend && cp .env.example .env && cargo run
cargo test
# frontend
cd ../frontend && npm install && npm run dev
# SDKs
cd ../sdk/typescript && npm install && npm test
cd ../python && pip install -e .[dev] && pytest
```

Try the core endpoint:

```bash
curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
```

(port per your `.env`)

## Preflight corridor cache

Preflight scores against a cached corridor table stored under the `preflight:corridors`
cache key. To keep preflight from scoring against stale data:

- Each ingestion batch invalidates `preflight:corridors` once it commits, so the next
  preflight request rebuilds the table from fresh data.
- The corridor cache TTL is capped at the ingestion interval, so the entry can never
  outlive the data it was derived from even if an invalidation is missed.
- Cache age is exposed as a metric so operators can alert when the corridor table is
  older than one ingestion cycle.

## Billing: USDC invoice confirmation

`confirm_invoice` grants the Pro plan only after verifying the on-chain payment on
Horizon. The check is deny-by-default: any input that does not match the invoice is
rejected rather than cleaned up. A transaction is accepted only when **all** of the
following hold:

- The transaction is loaded by hash and has `successful == true` (failed txs are
  rejected).
- It contains a payment operation to the configured billing account whose
  `asset_code == USDC` **and** `asset_issuer == <configured issuer>` (fake USDC or a
  wrong issuer is rejected).
- The delivered `amount >= invoice.amount` (underpayments are rejected).
- The transaction memo equals the invoice memo (memo is bound to the invoice).
- The transaction hash has not already been consumed by another invoice (reused tx
  hashes are rejected).

Path payments that deliver USDC to the billing account are accepted when the
destination asset and amount match. Payments split across multiple operations are
summed before the amount check.

## Preflight calibration report

Credibility of the preflight `hold` / `proceed` decisions depends on showing that `hold`
actually predicts failures. We publish a calibration report that joins each preflight
decision with the subsequently observed payment outcome on the same corridor and reports
weekly per-decision failure rates plus precision/recall for the `hold` decision.

- Report: [`docs/preflight-calibration.md`](docs/preflight-calibration.md)
- Generator: `backend/scripts/preflight_calibration.py` (see the report header for usage)

See the report for the data model, edge cases, and how to regenerate it from real data.
