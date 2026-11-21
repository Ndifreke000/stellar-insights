# Push notification delivery pipeline: outbox, receipts, token hygiene

> Design doc for issue **#2847**. Status: **proposed**, not yet implemented.

## Problem

Push notifications live in middleware modules. Delivery results are never processed, so invalid tokens are never removed. Push providers throttle senders that keep pushing to dead tokens.

### Provider note: FCM, not Expo

The issue mentions Expo push receipts. The mobile app is a **bare React Native app using `@react-native-firebase/messaging`** (see [`mobile/src/services/notifications.ts`](../mobile/src/services/notifications.ts) and `mobile/package.json`). It registers FCM tokens with `provider: 'fcm'`, and Expo isn't installed. This design therefore targets FCM (HTTP v1) and APNs, which are the providers `NotificationProvider` already models. It keeps a provider trait so an Expo adapter (`/push/send` plus `/push/getReceipts`, pruning on `DeviceNotRegistered`) can be added later without changing the pipeline. The token-hygiene rule is the same for every provider:

| Provider | "Token is dead" signal | Where it shows up |
| --- | --- | --- |
| FCM HTTP v1 | `404 NOT_FOUND` / `UNREGISTERED`, `400 INVALID_ARGUMENT` on token | Synchronous send response |
| APNs | `410 Unregistered`, `400 BadDeviceToken` | Synchronous send response |
| Expo (future) | `DeviceNotRegistered` | Async receipt fetched by `/getReceipts` 15+ minutes later |

## Current state

| File | Behaviour |
| --- | --- |
| [`backend/src/middleware/push_notification_service.rs`](../backend/src/middleware/push_notification_service.rs) | `process()` increments an in-memory counter and logs. **Nothing is ever sent.** |
| [`backend/src/middleware/push_notification_registration.rs`](../backend/src/middleware/push_notification_registration.rs) | Validates the G-address and checks the token is non-empty. `max_devices_per_account` is logged but **not enforced**. A random `registration_id` is generated and then thrown away. |
| [`backend/src/api/device_tokens.rs`](../backend/src/api/device_tokens.rs) | `POST /notifications/devices` upserts into `device_tokens` (migration `051`). `DELETE` and `GET /devices/{id}` exist. The network context is hard-coded to testnet. **No auth on any route**, so anyone can register a token against any account, list any account's tokens, or delete any token. |
| [`mobile/src/services/notifications.ts`](../mobile/src/services/notifications.ts) | Gets the FCM token, re-registers on refresh, deregisters on logout, and shows foreground messages with Notifee. It ignores `data.type` / collapse info. |

## Design

### 1. Move push out of middleware into `services/push/`

```
services/push/
  mod.rs          PushService: enqueue(), API used by alerting/watchlist code
  outbox.rs       outbox table access + claim/lease
  dispatcher.rs   background worker: claim → rate-limit → send → record → prune
  provider.rs     trait PushProvider { async fn send(&self, batch) -> Vec<SendOutcome> }
  fcm.rs, apns.rs provider adapters (expo.rs later)
  metrics.rs
```

`middleware/push_notification_*` are deleted. Validation moves into `PushService::register_device`, and `api/device_tokens.rs` calls the service.

### 2. Transactional outbox

```sql
CREATE TABLE push_outbox (
    id            TEXT PRIMARY KEY,
    account       TEXT NOT NULL,
    device_token  TEXT NOT NULL,
    provider      TEXT NOT NULL,
    collapse_key  TEXT,                 -- e.g. "corridor:USDC:NGN"
    payload       TEXT NOT NULL,        -- JSON {title, body, data}
    status        TEXT NOT NULL,        -- pending | sending | sent | failed | dropped
    attempts      INTEGER NOT NULL DEFAULT 0,
    next_attempt_at TEXT NOT NULL,
    lease_until   TEXT,
    lease_token   INTEGER,              -- fencing token, same pattern as #2811 job runs
    provider_message_id TEXT,
    last_error    TEXT,
    created_at    TEXT NOT NULL,
    sent_at       TEXT
);
CREATE INDEX idx_push_outbox_due ON push_outbox(status, next_attempt_at);
CREATE UNIQUE INDEX idx_push_outbox_collapse
    ON push_outbox(device_token, collapse_key) WHERE status = 'pending' AND collapse_key IS NOT NULL;
```

- Producers (corridor status alerts, watchlist changes) call `PushService::enqueue(account, notification)` **in the same DB transaction** as the state change that triggers it. This fan-out writes one row per active device token.
- The dispatcher claims due rows with a lease and a fencing token, so with several replicas each row is sent at most once per lease. It reuses the fencing helper from #2811.
- Retries on 5xx / 429 / timeouts use exponential backoff with jitter (honouring `Retry-After`), up to `max_retries` (existing `Config.max_retries`, default 3). After that the row is marked `failed`.
- Retention: `sent` / `dropped` rows are deleted after 7 days by the non-blocking retention job from #2912.

### 3. Delivery receipts and token pruning

For each send outcome:

| Outcome | Action |
| --- | --- |
| Success | `status = sent`, store `provider_message_id` |
| Dead token (table above) | `status = dropped`, **`DELETE FROM device_tokens WHERE device_token = ?`**, drop the other pending rows for that token, `push_tokens_pruned_total{provider,reason}` += 1 |
| Retryable | reschedule |
| Permanent payload error (`INVALID_ARGUMENT` on payload, `PayloadTooLarge`) | `status = failed`, log at error level. The token is **not** pruned. |

FCM and APNs report dead tokens at send time, so no separate receipt poller is needed for them. The `PushProvider` trait still has an optional `fetch_receipts(ids)` hook. A provider with async receipts (Expo) gets a `push_receipts` job that runs every 15 minutes over `sent` rows from the last 24 h that have a `provider_message_id`, and feeds the results into the same pruning path.

**Staleness pruning:** tokens with `updated_at` older than 270 days (FCM treats tokens inactive after 270 days) are removed by the retention job. The mobile app already re-registers on every launch and on `onTokenRefresh`, which bumps `updated_at`.

### 4. Per-user rate limits and collapse keys

- **Collapse:** each notification type sets a `collapse_key` (`corridor:{id}` for status changes). The partial unique index means a newer pending notification for the same device and key **replaces** the queued one (upsert) instead of stacking. The key is also passed to the provider (`android.collapse_key`, `apns-collapse-id`) so the device keeps only the latest one.
- **Rate limit:** a per-account token bucket in the dispatcher. Defaults are 10 notifications per hour and 50 per day, configurable through `PUSH_RATE_LIMIT_PER_HOUR` / `PUSH_RATE_LIMIT_PER_DAY`. Notifications over the limit are `dropped` with `reason=rate_limited`, but rows with a collapse key are deferred to the next window rather than dropped, so the user still gets the latest state.
- **Device cap:** actually enforce `max_devices_per_account` at registration by evicting the least recently updated token.

### 5. Securing the device-token API (prerequisite)

The endpoints must sit behind SEP-10/JWT auth. `account` comes from the authenticated subject, not the request body. `GET /devices/{account}` and `DELETE /devices/{token}` only work on the caller's own tokens. Without this, token pruning is pointless because anyone can register junk tokens or delete other users' tokens. Network context comes from the request (`NetworkContext` extractor), not a hard-coded testnet.

### 6. Metrics

Prometheus, following [prometheus-metrics.md](prometheus-metrics.md):

| Metric | Type | Labels |
| --- | --- | --- |
| `push_enqueued_total` | counter | `type` |
| `push_sent_total` | counter | `provider`, `result` (`success`/`retry`/`failed`/`dropped`) |
| `push_send_duration_seconds` | histogram | `provider` |
| `push_tokens_pruned_total` | counter | `provider`, `reason` (`unregistered`/`invalid`/`stale`) |
| `push_rate_limited_total` | counter | — |
| `push_outbox_pending` | gauge | — |
| `push_outbox_oldest_pending_seconds` | gauge | — (alert if > 300) |

Add a "Push delivery" row to the Grafana dashboard and an alert rule in `monitoring/` for oldest-pending age and failure ratio > 5%.

### Mobile changes

- Switch the foreground display to use `data.collapse_key` as the Notifee notification `id`, so updates replace each other on-device.
- No other client changes are needed. Token refresh and logout deregistration already exist.

## Tasks

- [ ] Auth + ownership checks on `/notifications/devices*`
- [ ] `services/push` module, `push_outbox` migration, remove `middleware/push_notification_*`
- [ ] FCM HTTP v1 adapter (service-account credentials via existing secrets management) and APNs adapter
- [ ] Dispatcher with lease/fencing, retries, dead-token pruning
- [ ] Collapse keys, per-account rate limit, device cap
- [ ] Stale-token pruning in the retention job
- [ ] Metrics + dashboard + alert
- [ ] Update `docs/API_DOCUMENTATION.md`, `docs/prometheus-metrics.md`, and add `push_outbox` to `docs/GDPR_DATA_INVENTORY.md` and `services/erasure.rs`
- [ ] Tests (below)

## Test plan

Using a `MockPushProvider` that returns scripted outcomes per token:

1. **Mocked receipts lead to pruned tokens (the acceptance test).** Register tokens A, B and C, then enqueue a notification for the account. The mock returns success for A, `UNREGISTERED` for B and `BadDeviceToken` for C. **Assert:** B and C are gone from `device_tokens`, A remains, and `push_tokens_pruned_total` is 2. A second enqueue only fans out to A.
2. Async-receipt path: the mock provider with `fetch_receipts` returns `DeviceNotRegistered`, and the receipt job prunes the token.
3. A retryable 503 then success leads to `attempts = 2` and `sent`. Repeated 503 leads to `failed` after `max_retries`.
4. Collapse: three pending status changes for the same corridor leave one row containing the latest payload.
5. Rate limit: the 11th notification in an hour is dropped (no collapse key) or deferred (with a collapse key).
6. Two dispatchers claiming at the same time send each row once (fencing).
7. Auth: registering with someone else's account, or deleting another account's token, returns 403.

## Acceptance criteria

- Delivery outcomes (and async receipts, for providers that have them) are processed
- Invalid tokens are pruned
- CI green (lint, type-check, tests)
- No new TODOs without a linked issue
