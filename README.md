# Stellar Insights

Stellar Insights is an analytics and preflight API for Stellar assets, with a Rust backend, a Next.js frontend, and TypeScript/Python SDKs.

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

Try the core endpoint: `curl -s "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"` (port per your `.env`).

## Rate limiting

All API routes are protected by a single rate limiting middleware. Every response — including successful `200` responses and rate-limited `429` responses — carries the standard IETF `RateLimit-*` headers so SDKs and clients can implement backoff without parsing error bodies:

| Header | Description |
| --- | --- |
| `RateLimit-Limit` | Maximum number of requests allowed in the current window. |
| `RateLimit-Remaining` | Number of requests remaining in the current window. |
| `RateLimit-Reset` | Seconds until the current window resets. |

When a request is rate limited, the response is a `429 Too Many Requests` with the same `RateLimit-*` headers (with `RateLimit-Remaining: 0`) plus a `Retry-After` header indicating how long the client should wait before retrying.

Example:

```bash
curl -i "http://localhost:8080/api/v1/preflight?source_asset=USDC&destination_asset=NGN&amount_usd=2500"
# HTTP/1.1 200 OK
# RateLimit-Limit: 60
# RateLimit-Remaining: 59
# RateLimit-Reset: 42
```

See the [IETF RateLimit header fields draft](https://datatracker.ietf.org/doc/draft-ietf-httpapi-ratelimit-headers/) for details on the header semantics.
