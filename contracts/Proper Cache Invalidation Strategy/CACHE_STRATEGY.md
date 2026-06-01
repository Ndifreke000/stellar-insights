# Cache Strategy

## Overview

The caching layer combines **TTL-based expiration** with **event-driven
invalidation**, **LRU eviction**, **startup warming**, and **metrics** to give
both low latency and data freshness.

---

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                     CacheManager<V>                      │
│                                                          │
│  HashMap<String, CacheEntry<V>>  ←── RwLock protected   │
│  broadcast::Sender<CacheInvalidationEvent>               │
│  AtomicU64 logical clock (LRU ordering)                  │
│  CacheMetrics (hits / misses / evictions / …)            │
│                                                          │
│  ┌─────────────────────────────────────────────────────┐ │
│  │  Background task (tokio::spawn)                     │ │
│  │  • Listens on broadcast channel                     │ │
│  │  • 60 s periodic TTL sweep                          │ │
│  │  • Handles PaymentDetected → invalidate corridor:*  │ │
│  │  • Handles AnchorStatusChanged → invalidate anchor:*│ │
│  │  • Handles AdminInvalidate → pattern match          │ │
│  │  • Handles MemoryPressure → LRU eviction loop       │ │
│  └─────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────┘
```

---

## Key Naming Convention

| Prefix        | Example                   | Invalidated by              |
|---------------|---------------------------|-----------------------------|
| `corridor:`   | `corridor:usdc-xlm`       | `PaymentDetected`           |
| `anchor:`     | `anchor:anchor-a`         | `AnchorStatusChanged`       |
| *(custom)*    | any pattern               | `AdminInvalidate { pattern }`|

---

## Invalidation Triggers

| Trigger                | Source                          | Effect                           |
|------------------------|---------------------------------|----------------------------------|
| `PaymentDetected`      | Payment listener / webhook      | Evict all `corridor:<id>:*` keys |
| `AnchorStatusChanged`  | SEP health-check / webhook      | Evict all `anchor:<id>:*` keys   |
| `AdminInvalidate`      | `DELETE /admin/cache/invalidate?pattern=` | Pattern match eviction |
| `TtlSweep`             | Internal 60 s ticker            | Remove expired entries           |
| `MemoryPressure`       | `POST /admin/cache/evict-lru?target=` | LRU loop until `target` size |

---

## Cache Warming

On startup, call `warm_corridor_cache` and `warm_anchor_cache` **before**
serving traffic:

```rust
// main.rs
warm_corridor_cache(&corridor_cache).await;
warm_anchor_cache(&anchor_cache).await;
```

Each warming function fetches the top N records from the DB/RPC and inserts
them with the default TTL.  Replace the stub vectors with real queries.

---

## LRU Eviction

- Every `get` stamps the entry with the current value of an `AtomicU64`
  logical clock.
- When `store.len() > capacity`, the entry with the **smallest** `last_used`
  counter is evicted.
- LRU is also triggered explicitly via `MemoryPressure` events from the admin
  endpoint.

---

## Metrics

`CacheManager::metrics()` returns a `CacheMetrics` snapshot:

| Field           | Meaning                              |
|-----------------|--------------------------------------|
| `hits`          | Successful cache reads               |
| `misses`        | Reads that fell through to the source|
| `invalidations` | Entries evicted by any strategy      |
| `evictions`     | Entries evicted by LRU specifically  |
| `warm_ups`      | Entries loaded during warming        |
| `current_size`  | Live entries at snapshot time        |
| `hit_rate()`    | `hits / (hits + misses)`             |

Expose via `GET /admin/cache/metrics`.

---

## Admin Endpoints

| Method   | Path                               | Action                                |
|----------|------------------------------------|---------------------------------------|
| `GET`    | `/admin/cache/metrics`             | Return `CacheMetrics` JSON            |
| `DELETE` | `/admin/cache/invalidate?pattern=` | Evict keys matching pattern           |
| `DELETE` | `/admin/cache/flush`               | Flush entire cache                    |
| `POST`   | `/admin/cache/evict-lru?target=`   | LRU-evict until `target` entries      |

> **Security**: these endpoints should be protected by an admin auth middleware
> (not included here – add your own layer).

---

## Adding a New Invalidation Trigger

1. Add a variant to `CacheInvalidationEvent` in `cache.rs`.
2. Handle it in the `match event` block of the background task.
3. Optionally add an `InvalidationRule` to `invalidation::default_rules()`.
4. Publish the event wherever the underlying data changes:
   ```rust
   cache.publish_event(CacheInvalidationEvent::YourNewEvent { ... });
   ```
