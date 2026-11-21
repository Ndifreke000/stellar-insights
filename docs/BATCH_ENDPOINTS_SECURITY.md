# Batch Endpoints & Field Selection Security Architecture

This document records the security audit and hardening for batch request dispatch (`POST /api/v1/batch`, `POST /api/v1/preflight/batch`) and response field selection (`?fields=...`) implemented for Issue #2832.

---

## 1. Threat Model & Middleware Execution Trace

Previously, `BatchEndpoints` and `FieldSelectionParameter` existed as isolated helpers without full per-sub-request enforcement:

- **Authorization bypass risk:** Dispatching sub-requests internally without re-executing `auth_middleware`, `AdminUser`, and `ip_whitelist_middleware`—or allowing sub-request payloads to supply `Authorization` / `X-Forwarded-For` headers—would allow a non-admin or non-whitelisted caller to reach `/admin/*` endpoints.
- **Rate-limit amplification:** Charging only `1` unit for an outer `POST /api/v1/batch` or `POST /api/v1/preflight/batch` carrying `100` items creates a 100× amplification vector against per-IP and per-API-key rate limits.
- **Cost & recursion DoS:** Unbounded batch sizes, unpriced expensive sub-requests (exports, analytics, writes), path traversal (`..`, `%2e%2e`), or recursive `/batch` calls could exhaust backend resources.
- **Sensitive field leakage:** Unconstrained `?fields=...` selection could allow probing or extracting secret or admin-only fields.

### Hardened Sub-Request Execution Trace

Every `POST /api/v1/batch` call follows this exact pipeline:

1. **Outer middleware layer:** `rate_limit_middleware` (marks `OuterRateLimitCharged`), `api_key_rate_limit_middleware`, `api_versioning_middleware`, `request_id_middleware`.
2. **Outer identity extraction (`BatchCallerIdentity`):** Captures `Authorization`, verified client IP (`crate::client_ip::client_ip`), peer `ConnectInfo<SocketAddr>`, and verified JWT claims (including revocation check via `resolve_optional_auth_user`).
3. **Boundary validation (`BatchDispatcher::validate_batch`):**
   - Rejects empty batches (`EMPTY_BATCH`, 400) and batches exceeding `max_batch_size` (default `100`, `BATCH_TOO_LARGE`, 400).
   - Validates HTTP method allowlist (`GET`, `POST`, `PUT`, `PATCH`, `DELETE`).
   - Canonicalizes sub-request paths and rejects schemes (`://`), `//`, backslashes, control chars, dot-segment traversal (`..`, `%2e%2e`), and recursive batch targets (`/batch`, `/preflight/batch`).
   - Caps per-sub-request JSON body size (`max_subrequest_body_bytes`, default 64 KiB) and aggregate weighted cost (`max_total_cost`, default `100`, `BATCH_COST_EXCEEDED`, 400).
4. **Per-sub-request enforcement loop:**
   - **Rate limiting:** Debits 1 unit per sub-request from the caller's rate-limit bucket (and per-API-key bucket when an API key is presented), ensuring a batch of `N` counts as exactly `N` units. Excess items receive sub-request status `429`.
   - **Admin IP whitelist:** For any `/admin` path, checks `IpWhitelistConfig::is_allowed(&identity.client_ip)` fail-closed before dispatch (`403 ADMIN_IP_NOT_WHITELISTED`).
   - **Header sanitization:** Strips `Authorization`, `Cookie`, `X-Forwarded-For`, `X-Real-IP`, `Forwarded`, `Host`, `X-API-Key`, and synthetic auth headers from sub-request payloads, then force-injects the verified outer `Authorization` header and `ConnectInfo<SocketAddr>`.
   - **Full inner router dispatch:** Executes `sub_router.clone().oneshot(sub_req)` (constructed from `v1_router` *without* `/batch` itself), re-running `auth_middleware`, `AdminUser`, `sep10_auth_middleware`, `api_versioning_middleware`, and `field_selection_middleware`.
   - **Role-aware field selection:** Validates requested fields against `FieldAccessPolicy` and filters only `2xx` JSON responses—never altering `401`/`403`/`429` error responses.

---

## 2. Cost Weights & Caps

| Sub-Request Type | Weight |
|------------------|--------|
| `GET` / `HEAD` (standard route) | `1` |
| `POST` / `PUT` / `PATCH` / `DELETE` | `3` |
| Expensive routes (`/export/*`, `/analytics/*`, `/preflight*`, `/admin/*`) | `+4` surcharge |

Defaults (`BatchConfig::default()`):
- `max_batch_size`: `100`
- `max_total_cost`: `100`
- `max_subrequest_body_bytes`: `65,536` (64 KiB)
- `deny_nested_batch`: `true`

`POST /api/v1/preflight/batch` also debits `items.len()` units from `RateLimiter` (both client bucket and API-key bucket when applicable).

---

## 3. Field Selection Authorization Policy

`FieldAccessPolicy` enforces two tiers on both `?fields=...` query parameters and batch `fields` properties:

1. **Always forbidden (all roles, `403 FORBIDDEN_FIELD_SELECTION` + stripped from 2xx JSON):**
   `password`, `password_hash`, `key_hash`, `secret`, `encrypted_secret`, `totp_secret`, `private_key`, `signing_key`, `jwt_secret`, `refresh_token_hash`, `api_key_hash`, `salt`.
2. **Admin-only (`AuthUser.is_admin == true` required, `403 FORBIDDEN_FIELD_SELECTION` for non-admin):**
   `added_by_user_id`, `ip_address`, `session_id`, `stack_trace`, `audit_hash`, `device_user_agent`.
