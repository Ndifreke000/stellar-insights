# API Pagination

Every list endpoint accepts the same query parameters and returns the same envelope.
Implementation: `backend/src/pagination.rs` (server), `frontend/src/lib/api/pagination.ts` (client).

## Request

| Parameter | Type   | Description |
|-----------|--------|-------------|
| `limit`   | int    | Page size. Clamped to `1..=max` (per-endpoint default and max below). |
| `cursor`  | string | Opaque cursor copied from `pagination.next_cursor` / `prev_cursor`. |
| `offset`  | int    | **Deprecated.** Still accepted; ignored when `cursor` is present. |

Cursors are opaque. Don't parse or build them on the client. The encoding can change
without a version bump. An invalid cursor returns `400` with code `INVALID_CURSOR`.

## Response

```json
{
  "data": [ ... ],
  "pagination": {
    "limit": 50,
    "total": 312,
    "has_next": true,
    "has_prev": false,
    "next_cursor": "djE6bzo1MA",
    "prev_cursor": null,
    "offset": 0,
    "next_offset": 50,
    "prev_offset": null
  },
  "links": {
    "self": "/api/corridors?limit=50",
    "next": "/api/corridors?limit=50&cursor=djE6bzo1MA",
    "prev": null
  }
}
```

- `total` is `null` when an endpoint can't count cheaply (upstream-proxied or append-only data).
  Use `has_next` to decide whether to fetch more.
- `links.next` / `links.prev` keep all your other query parameters (filters, sort) and swap in the new cursor.
- `offset`, `next_offset` and `prev_offset` are kept for existing clients and will be removed later.

## Endpoints

| Endpoint | Default / max `limit` | `total` | `prev_cursor` |
|----------|-----------------------|---------|---------------|
| `GET /api/corridors` | 50 / 200 | yes | yes |
| `GET /api/anchors` | 50 / 200 | yes | yes |
| `GET /api/governance/proposals` | 20 / 100 | yes | yes |
| `GET /api/analytics/contract-events` | 50 / 200 | `null` | yes |
| `GET /api/rpc/payments` | 20 / 200 | `null` | `null` (Horizon paging token; newest first, forward only) |

Results are always in a stable order (with a unique tie-breaker), so paging never skips or repeats items.
`/api/corridors` sorts by `sort_by` (`success_rate`, `volume`/`liquidity`, `health_score`), descending.

## Example

```bash
curl 'http://localhost:8080/api/corridors?limit=10'
# → pagination.next_cursor = "djE6bzoxMA"
curl 'http://localhost:8080/api/corridors?limit=10&cursor=djE6bzoxMA'
```

## Adding a paginated endpoint

```rust
use axum::extract::OriginalUri;
use crate::pagination::{PaginatedResponse, PaginationParams};

async fn list_things(
    OriginalUri(uri): OriginalUri,
    Query(q): Query<ListThingsQuery>, // has limit / cursor / offset fields
) -> ApiResult<Json<PaginatedResponse<Thing>>> {
    let page = PaginationParams { limit: q.limit, cursor: q.cursor, offset: q.offset }
        .resolve(50, 200)?;
    let items = db.list_things(page.limit, page.offset).await?; // ORDER BY must be deterministic
    let total = db.count_things().await?;
    Ok(Json(PaginatedResponse::from_page(items, total, page).with_links(&uri)))
}
```

- No cheap count? Fetch `page.limit + 1` rows and use `PaginatedResponse::from_probe(rows, page)`.
- Proxying an upstream paging token? Use `PaginatedResponse::from_tokens(items, limit, next, prev)`.
- Don't `#[serde(flatten)]` `PaginationParams` into a query struct: `serde_urlencoded` can't
  deserialize numbers through `flatten`. Declare the three fields and build `PaginationParams` from them.
- Add the instantiation to the `aliases(...)` list on `PaginatedResponse` and register it in
  `openapi.rs` so the spec shows the concrete item type.

On the frontend, type responses as `PaginatedResponse<T>` and use `getNextCursor` with `useInfiniteQuery`
when you need incremental loading.
