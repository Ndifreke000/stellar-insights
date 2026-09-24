# API HTTP Caching (Cache-Control, ETag, 304)

Tracking issue: #2412

## Summary

HTTP caching is implemented in [`backend/src/http_cache.rs`](../backend/src/http_cache.rs)
through a single helper, `cached_json_response`. A handler that calls it returns:

| Header          | Value                                                        |
|-----------------|--------------------------------------------------------------|
| `Cache-Control` | `public, max-age=<ttl>`                                      |
| `ETag`          | Strong ETag: `"<sha256 of JSON body, hex>"`                  |
| `Last-Modified` | When the ETag for this resource key was last seen to change  |
| `Content-Type`  | `application/json`                                           |

It returns `304 Not Modified` with an empty body, keeping the same headers, when either:

- `If-None-Match` matches the current ETag (it accepts weak `W/` prefixes, comma lists and `*`), or
- `If-Modified-Since` is at or after `Last-Modified`.

## Endpoints that use it

| Handler                                  | TTL source                    | Default TTL |
|------------------------------------------|-------------------------------|-------------|
| `corridors::list_corridors`              | `cache.config.get_ttl("corridor")`  | 300 s |
| `corridors::get_corridor_detail`         | `cache.config.get_ttl("corridor")`  | 300 s |
| `anchors::get_anchors`                   | `cache.config.get_ttl("anchor")`    | 600 s |
| `metrics::metrics_overview`              | `cache.config.get_ttl("dashboard")` | 60 s  |
| `cache_stats::get_cache_stats`           | hard-coded                          | 30 s  |
| `cost_calculator::estimate_costs`        | `DEFAULT_CACHE_TTL_SECONDS`         | 60 s  |

The anchor and corridor handlers are mounted under `/api/v1` (for example `/api/v1/corridors`).
All other endpoints send **no** caching headers.

## Configuring TTLs

TTLs come from `CacheConfig::from_env()` in [`backend/src/cache.rs`](../backend/src/cache.rs):

| Env var                       | Applies to         | Default |
|-------------------------------|--------------------|---------|
| `CACHE_CORRIDOR_METRICS_TTL`  | corridor endpoints | 300     |
| `CACHE_ANCHOR_DATA_TTL`       | anchor endpoints   | 600     |
| `CACHE_DASHBOARD_STATS_TTL`   | metrics overview   | 60      |

The same value controls both the server-side Redis/in-memory cache and `max-age`.

## Adding caching to another endpoint

1. Add `headers: HeaderMap` to the handler's extractors.
2. Choose a stable resource key that includes every query parameter that changes the body
   (see `generate_corridor_list_cache_key`).
3. Return the payload through the helper:

```rust
let ttl = cache.config.get_ttl("corridor");
let response = crate::http_cache::cached_json_response(&headers, &cache_key, &payload, ttl)?;
Ok(response)
```

Guidelines:

- **Do not** cache per-user, authenticated or admin responses with `public`. If needed, add a
  `private` variant to the helper first.
- Mutations (`POST`/`PUT`/`DELETE`) should never use this helper.
- Choose a TTL no longer than the refresh interval of the data behind the endpoint.

## Manual verification

```bash
# 1. Fresh response – note the ETag
curl -si http://localhost:8080/api/v1/corridors | grep -iE 'HTTP/|cache-control|etag|last-modified'

# 2. Conditional request – expect 304
ETAG=$(curl -si http://localhost:8080/api/v1/corridors | awk -F': ' 'tolower($1)=="etag"{print $2}' | tr -d '\r')
curl -si -H "If-None-Match: $ETAG" http://localhost:8080/api/v1/corridors | head -1
```

## Known limitations / follow-ups

- `Last-Modified` metadata lives in a per-process in-memory map (`CACHE_METADATA`). It resets
  on restart and differs between replicas, so `If-Modified-Since` is less reliable than
  `If-None-Match` behind a load balancer. The ETag is content-derived, so it stays consistent
  across replicas.
- The metadata map is never pruned. This is fine for the current fixed key set, but it would
  grow on endpoints with high-cardinality query keys.
- The ETag is computed after serialization, so a `304` saves bandwidth but not the database
  or cache lookup.
- The server does not send `Vary` headers. Add `Vary: Accept-Encoding` if compression varies
  by client.
