# Design note: role-based admin access with least privilege

Tracking issue: #2859
Status: proposed

## Problem

Admin access is all or nothing. A user either has `users.is_admin = true` and can do every
admin action, or has none. Support staff who only need to look up an account get the same
power as someone running backfills or editing the IP allowlist. We need granular roles
(support, finance, operator and so on), enforced on the server, audited, and reflected in
the UI.

## Current state

Code checked on `main` at `d60acbb2`. Writing this note turned up several gaps that the
RBAC work has to close, not only the missing roles:

| Finding | Where | Severity |
|---|---|---|
| `/admin/backfill`, `/admin/backfill/resume` and `/admin/backfill/status` take no auth extractor, and no auth layer is applied | [`api/backfill.rs`](../../backend/src/api/backfill.rs) `routes()`, mounted in [`main.rs`](../../backend/src/main.rs) `.nest("/admin", admin_routes)` | **High.** Anyone can start a backfill |
| `ip_whitelist_middleware` is defined but never applied to any router | [`ip_whitelist_middleware.rs`](../../backend/src/ip_whitelist_middleware.rs) | **High.** The IP allowlist described in [IP_WHITELIST.md](../IP_WHITELIST.md) is not enforced |
| `AdminAuditLogger::log_action` / `log_action_with_context` have no callers | [`admin_audit_log.rs`](../../backend/src/admin_audit_log.rs) | **High.** No privileged action is audited today |
| `log_action` takes `prev_hash` from the caller | same | Medium. Two concurrent writers can fork the hash chain |
| `/api/v1/db/slow-queries` and `/db/index-report` are on the public router | [`api/v1/mod.rs`](../../backend/src/api/v1/mod.rs) | Medium. Exposes query text and schema details |
| `is_admin` is copied from the refresh token's claims, so a demotion only takes effect at the next full login | [`auth.rs`](../../backend/src/auth.rs) `refresh` | Medium. Revocation is slow |
| The `/internal/monitoring` frontend page is gated by a shared `INTERNAL_SECRET`, not by user identity | `frontend/src/app/[locale]/internal/monitoring/page.tsx` | Low. Not attributable to a person |

Working pieces this design reuses: `AuthUser` and `AdminUser` extractors in
[`auth_middleware.rs`](../../backend/src/auth_middleware.rs), `auth_middleware` on the
`/api/v1/admin/*` groups, `IpWhitelistService`, and the hash-chained `admin_audit_log`
table ([AUDIT_LOG.md](../AUDIT_LOG.md)).

**Recommendation:** fix the first two findings in a small, separate PR before the RBAC
work starts. Put `auth_middleware` + `AdminUser` on the backfill router and apply
`ip_whitelist_middleware` to every admin group. These are exposures today and should not
wait one to two weeks for this design.

## Design

### Roles and permissions

Permissions are `resource:action` strings defined as a Rust enum, so a typo fails to
compile. Roles are named sets of permissions. A user can hold several roles, and their
effective permissions are the union.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, EnumIter)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    UsersRead, UsersWrite,               // users:read, users:write
    RolesManage,                         // roles:manage
    BillingRead, BillingWrite,           // billing:read, billing:write
    BackfillRun, BackfillRead,           // backfill:run, backfill:read
    ContractEventsGapsResolve,           // contract_events:gaps_resolve (#2858)
    IpAllowlistRead, IpAllowlistWrite,   // ip_allowlist:read, ip_allowlist:write
    AuditLogRead, AuditLogVerify,        // audit_log:read, audit_log:verify
    DbDiagnosticsRead,                   // db_diagnostics:read
    SnapshotsGenerate,                   // snapshots:generate
    ExportsReadAny,                      // exports:read_any (#2856)
}
```

Permissions matrix. This is the source of truth, and it lives in code as
`services/rbac/matrix.rs`:

| Permission | `support` | `finance` | `operator` | `auditor` | `security_admin` | `super_admin` |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| `users:read` | ✓ | ✓ | | ✓ | ✓ | ✓ |
| `users:write` | | | | | ✓ | ✓ |
| `roles:manage` | | | | | | ✓ |
| `billing:read` | ✓ | ✓ | | ✓ | | ✓ |
| `billing:write` | | ✓ | | | | ✓ |
| `backfill:read` | | | ✓ | ✓ | | ✓ |
| `backfill:run` | | | ✓ | | | ✓ |
| `contract_events:gaps_resolve` | | | ✓ | | | ✓ |
| `snapshots:generate` | | | ✓ | | | ✓ |
| `db_diagnostics:read` | | | ✓ | | | ✓ |
| `exports:read_any` | ✓ | | | ✓ | | ✓ |
| `ip_allowlist:read` | | | ✓ | ✓ | ✓ | ✓ |
| `ip_allowlist:write` | | | | | ✓ | ✓ |
| `audit_log:read` | | | | ✓ | ✓ | ✓ |
| `audit_log:verify` | | | | ✓ | ✓ | ✓ |

The rules behind the matrix:

- **Separation of duties.** Nobody but `super_admin` can both perform an action and read
  the audit log of it.
- `roles:manage` belongs to `super_admin` only.
- A user cannot grant a role they do not hold. Removing the last `super_admin` is refused.

### Data model

Migration `052_create_admin_rbac.sql`:

```sql
CREATE TABLE IF NOT EXISTS admin_role_assignments (
    user_id     TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role        TEXT NOT NULL,          -- validated against the Role enum in code
    granted_by  TEXT NOT NULL,
    granted_at  TEXT NOT NULL,
    expires_at  TEXT,                   -- optional time-boxed access
    PRIMARY KEY (user_id, role)
);
CREATE INDEX IF NOT EXISTS idx_admin_role_assignments_user ON admin_role_assignments (user_id);

-- Backfill: every existing is_admin user becomes super_admin, so nobody loses access on deploy.
INSERT OR IGNORE INTO admin_role_assignments (user_id, role, granted_by, granted_at)
SELECT id, 'super_admin', 'migration:052', CURRENT_TIMESTAMP FROM users WHERE is_admin = 1;
```

The role-to-permission matrix stays in code, not in the database. Changing what a role can
do is then a reviewed code change with tests, not a runtime edit. `users.is_admin` is kept
for one release as a read-only mirror ("has any admin role"), then removed.

### Enforcement

**Route metadata.** Every privileged route states its permission next to its definition,
using a small router extension:

```rust
Router::new()
    .route("/backfill",        post(start_backfill).require(Permission::BackfillRun))
    .route("/backfill/status", get(get_backfill_status).require(Permission::BackfillRead))
```

`require` wraps the handler in a `RequirePermission` layer. Underneath, it is
`axum::middleware::from_fn_with_state` holding the permission.

**Middleware order** for every admin group, outermost first:

1. `ip_whitelist_middleware`: the network gate. Once this work lands, it is actually applied.
2. `auth_middleware`: resolves `AuthUser`.
3. `RequirePermission(p)`:
   - loads the user's effective permissions through `RbacService::permissions_for(user_id)`,
     which is cached per user for 30 seconds and cleared immediately on any role change,
   - returns `403` in the standard `ApiError` shape with
     `code = "FORBIDDEN_PERMISSION"` and `details.required = "backfill:run"` when the
     permission is missing,
   - inserts an `Authorized { user, permission }` extension so handlers and the audit
     layer know what was checked.

Permissions are looked up from the database (through the cache) rather than from JWT
claims. A revoked role then stops working within 30 seconds, which also fixes the
"demotion waits for re-login" finding.

**Deny by default.** A test walks the router and fails if any route under `/admin` or
`/api/v1/admin` has no `require(...)`. A newly added admin route therefore cannot ship
without a permission.

**Handlers stay thin.** Handlers do not check roles. Ownership-style checks (for example
`exports:read_any` versus reading your own export) go through
`RbacService::can(user, Permission)`, so they are testable without HTTP.

### Auditing every privileged action

An `AuditLayer` sits inside `RequirePermission` and writes one `admin_audit_log` row per
request on a privileged route:

- `action` is the permission (`backfill:run`). `resource` is the matched route plus path
  params (for example `/admin/ip-whitelist/10.0.0.0/8`).
- `status` is `success`, `denied` (a 403 from the permission check) or `failed` (a 4xx or
  5xx from the handler).
- `details` holds the request body with sensitive fields redacted (a per-route allowlist
  of logged fields, not a denylist), the response status and the duration.
- Session id, IP and user agent come from the existing `log_action_with_context`.
- **Denied attempts are logged too.** That record is what exposes probing.

Read-only permissions (`*:read`) are logged as well. Seeing who looked at what is the
point of a support role.

**Hash-chain fix.** `log_action` stops taking `prev_hash` from callers. It reads the last
hash and inserts inside one `BEGIN IMMEDIATE` transaction, which takes SQLite's write lock
and so serialises writers. That makes the chain safe under concurrency.
`verify_integrity` is unchanged.

**Audit failure policy.** For write permissions, if the audit insert fails, the request
fails with `503` and the action does not run. The audit row is written *before* the handler
runs (`status = 'pending'`) and updated afterwards. For read permissions, an audit failure
is logged and counted, and the request still proceeds.

### Admin API

| Method and path | Permission |
|---|---|
| `GET /api/v1/admin/me/permissions` | any authenticated admin. Returns `{ roles, permissions }` |
| `GET /api/v1/admin/roles` | `roles:manage`. Returns the matrix (read from code) |
| `GET /api/v1/admin/users/{id}/roles` | `users:read` |
| `PUT /api/v1/admin/users/{id}/roles/{role}` | `roles:manage`. Optional `{ expires_at }` |
| `DELETE /api/v1/admin/users/{id}/roles/{role}` | `roles:manage` |

Existing routes get these permissions: backfill (`backfill:*`), IP allowlist
(`ip_allowlist:*`), audit log (`audit_log:*`), `POST /api/snapshots/generate`
(`snapshots:generate`), and `/db/slow-queries` plus `/db/index-report`, which move off the
public router to `db_diagnostics:read`. `/db/pool-metrics` stays public because the health
checks use it.

### UI

- On load, the admin area calls `/api/v1/admin/me/permissions` and keeps the result in a
  `usePermissions()` hook.
- `<Can permission="backfill:run">…</Can>` hides any control the user lacks. Nav entries
  are filtered the same way, using the feature-flag pattern from #2686.
- The UI is a convenience, not the enforcement point. The server still returns 403 for
  everything. A 403 on a hidden action triggers a permission refetch, because roles may
  have changed.
- `/internal/monitoring` moves from `INTERNAL_SECRET` to `db_diagnostics:read`. The shared
  secret is kept only as a fallback for local development.

## Failure modes

| Failure | Behaviour |
|---|---|
| Role lookup DB error | Fail closed with `503`. Never fall back to allowing access |
| Cache shows a just-revoked role | At most 30 seconds. Revocation on the same instance clears the cache immediately. Across instances, the TTL is the bound (or use Redis pub/sub, see Deferred) |
| Audit insert fails on a write action | `503`. The action does not run |
| Last `super_admin` removed | `409 LAST_SUPER_ADMIN` |
| Self-escalation (granting a role the caller does not hold) | `403 FORBIDDEN_PERMISSION` |
| Expired role assignment | Ignored by `permissions_for`. A daily job deletes expired rows and audits each deletion |
| Unknown role string in the DB (for example after a rollback) | Ignored with a warning. It grants nothing |

## Test plan

1. **Matrix tests, one per role** (the issue's test plan). For every `(role, route)` pair,
   generated from the matrix and the router inventory, assert `2xx/4xx-not-403` when the
   role has the permission and `403 FORBIDDEN_PERMISSION` when it does not. The test is
   generated, so adding a route or a permission updates it automatically.
2. **Deny-by-default router test.** Every `/admin` and `/api/v1/admin` route has a `require`.
3. **Regression: backfill requires auth.** An unauthenticated `POST /admin/backfill`
   returns `401`. Today it returns `202`.
4. **Regression: IP allowlist enforced.** A request from an address outside the allowlist
   returns `403` before auth runs.
5. **Union of roles.** A user with `support` and `finance` gets both sets, and nothing else.
6. **Revocation.** Remove a role. The next request on the same instance returns `403`.
7. **Audit rows.** A success, a handler failure and a denied attempt each write exactly one
   row with the right status. Redacted fields do not appear in `details`.
8. **Concurrent audit writes.** 50 parallel privileged requests, then `verify_integrity`
   returns valid. This is the regression test for the `prev_hash` fork.
9. **Audit failure blocks writes.** With the audit table made read-only, `POST /admin/backfill`
   returns `503` and no backfill starts.
10. **Last super_admin** cannot be removed. **Self-escalation** is refused.
11. **Frontend.** `<Can>` hides controls by permission set, and nav filtering works.

## Rollout

1. Separate hotfix PR: auth on backfill, and `ip_whitelist_middleware` applied.
2. Migration and `RbacService`, with every existing admin mapped to `super_admin`. No
   behaviour change.
3. Enforcement and audit layers turned on. Assign narrower roles to staff and remove
   `super_admin` where it is not needed.
4. UI gating.
5. One release later, drop `users.is_admin`.

## Docs to update

- [AUDIT_LOG.md](../AUDIT_LOG.md): what is logged per request, the failure policy, and the
  chain-locking change.
- [IP_WHITELIST.md](../IP_WHITELIST.md): which routers the middleware actually covers.
- New `docs/ADMIN_ROLES.md`: the matrix, how to grant roles, and the break-glass procedure.
- `openapi.json`: add an `x-required-permission` extension on each admin operation.

## Deferred

- Per-resource scoping (for example "support for region X only"). Global permissions
  are enough for now.
- Redis pub/sub for cross-instance cache invalidation. Use it if 30 seconds of staleness
  turns out to be too long.
- Just-in-time elevation with approval. `expires_at` covers time-boxed grants for now.
- Requiring 2FA ([TWO_FACTOR_AUTHENTICATION.md](../TWO_FACTOR_AUTHENTICATION.md)) for any
  write permission. Recommended as an immediate follow-up.
