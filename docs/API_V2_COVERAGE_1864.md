# API Versioning — v1 vs v2 Route Coverage

_Audit for issue #1864._

---

## Current state

`backend/src/api/v1/mod.rs::routes()` mounts the router three ways:

```
/api/v1/**   — versioned v1 prefix
/api/v2/**   — v2 stub (see below)
/**          — unversioned root (backward-compat alias for v1)
```

### v1 route inventory

The following routes are mounted under `/api/v1` (and mirrored at the root):

#### Anchors
| Method | Path | Handler |
|---|---|---|
| GET | `/anchors` | `anchors::get_anchors` (cached) |
| GET | `/anchors/{id}` | `anchors::get_anchor` |
| GET | `/anchors/account/{stellar_account}` | `anchors::get_anchor_by_account` |
| GET | `/anchors/{id}/assets` | `anchors::get_anchor_assets` |
| POST | `/anchors` | `anchors::create_anchor` (auth) |
| PUT | `/anchors/{id}/metrics` | `anchors::update_anchor_metrics` (auth) |
| POST | `/anchors/{id}/assets` | `anchors::create_anchor_asset` (auth) |

#### Corridors
| Method | Path | Handler |
|---|---|---|
| GET | `/corridors` | `corridors::list_corridors` (cached) |
| GET | `/corridors/{corridor_key}` | `corridors::get_corridor_detail` (cached) |
| POST | `/corridors` | `corridors::create_corridor` (auth) |
| PUT | `/corridors/{id}/metrics-from-transactions` | `corridors::update_corridor_metrics_from_transactions` (auth) |

#### Analytics / Misc
| Method | Path | Handler |
|---|---|---|
| GET | `/analytics/muxed` | `anchors::get_muxed_analytics` |
| GET | `/analytics/**` | `analytics_dashboard::routes` |

#### Infrastructure
| Method | Path | Handler |
|---|---|---|
| GET | `/health` | `handlers::health_check` |
| GET | `/db/pool-metrics` | `handlers::pool_metrics` |
| GET | `/metrics/**` | `api::metrics::routes` |
| GET | `/jobs/**` | `handlers::job_monitoring routes` |
| GET | `/jobs/status` | `job_monitoring::get_job_status` |
| GET | `/jobs/health` | `job_monitoring::get_job_health` |
| GET | `/jobs/metrics` | `job_monitoring::get_job_metrics` |

#### Export
| Method | Path | Handler |
|---|---|---|
| GET | `/export/corridors` | `export::export_corridors` |
| GET | `/export/anchors` | `export::export_anchors` |
| GET | `/export/payments` | `export::export_payments` |

#### RPC
| Method | Path | Handler |
|---|---|---|
| GET | `/rpc/health` | `rpc::rpc_health_check` |
| GET | `/rpc/ledger/latest` | `rpc::get_latest_ledger` |
| GET | `/rpc/payments` | `rpc::get_payments` |
| GET | `/rpc/payments/account/{account_id}` | `rpc::get_account_payments` |
| GET | `/rpc/trades` | `rpc::get_trades` |
| GET | `/rpc/orderbook` | `rpc::get_order_book` |

#### Services (nested routers)
| Prefix | Module |
|---|---|
| `/fee-bumps/**` | `fee_bump::routes` |
| `/account-merges/**` | `account_merges::routes` |
| `/liquidity-pools/**` | `liquidity_pools::routes` |
| `/prices/**` | `price_feed_api::routes` |
| `/cost-calculator/**` | `cost_calculator::routes` |
| `/cache/stats/**` | `cache_stats::routes` |
| `/webhooks/**` (auth) | `webhooks::routes` |

#### Auth / OAuth
| Prefix | Module |
|---|---|
| `/auth/**` | `auth::routes` (via oauth module) |
| `/oauth/**` | `oauth::routes` |

#### SEP-24 (mounted at root, not under `/api/v1`)
| Prefix | Module |
|---|---|
| `/sep24/**` | `sep24_proxy::routes` |

#### Version info
| Method | Path |
|---|---|
| GET | `/api/version` |

---

### v2 route inventory

```rust
fn v2_routes() -> Router {
    Router::new().route("/status", get(v2_not_implemented))
}
```

v2 currently exposes **one route**:

| Method | Path | Response |
|---|---|---|
| GET | `/api/v2/status` | `{"message":"API v2 is reserved for future releases","status":"not_implemented"}` |

**v2 is a stub.** It is not a subset or superset of v1 — it is a placeholder.

---

## Intended relationship between v1 and v2

Based on the code and existing architecture documentation
(`docs/architecture/API_VERSIONING.md`), the intended model is:

- **v1** is the current stable API. It is preserved unversioned at the root for
  backward compatibility with existing clients.
- **v2** will introduce breaking changes (response-shape changes, pagination
  model changes, removal of deprecated fields) that cannot be introduced in v1
  without breaking clients.
- v1 and v2 are intended to **coexist** while clients migrate. v1 has a
  documented sunset date of `2025-01-01` in the `get_api_version()` handler,
  though that date has already passed — this needs updating.
- v2 is **not** meant to silently 404 on all v1 routes; it will eventually be
  a complete superset.

---

## Gap analysis

All v1 routes are **missing from v2**. Since v2 is explicitly a future-work
stub this is expected, but it means:

1. Any client targeting `/api/v2/*` (other than `/api/v2/status`) will receive
   a 404, not a helpful "not implemented" response.
2. There is no automated test asserting that v2 returns the correct stub
   response for unknown paths.

---

## Follow-up issues filed

- **#1864-a** (this document): document the gap — done.
- The missing v2 routes are intentional today. When v2 implementation begins,
  the `v2_routes()` function in `backend/src/api/v1/mod.rs` is the only place
  that needs to grow.
- The v1 sunset date (`2025-01-01`) in `get_api_version()` should be updated to
  a realistic future date once v2 is ready. See `backend/src/api/v1/mod.rs:56`.

---

## v2 catch-all recommendation

To prevent silent 404s on `/api/v2/*` today, a fallback handler should be added:

```rust
fn v2_routes() -> Router {
    Router::new()
        .route("/status", get(v2_not_implemented))
        .fallback(v2_not_implemented)   // return structured JSON, not a bare 404
}
```

This is a one-line change that makes the API friendlier for clients that
accidentally hit v2 endpoints before they are implemented.
