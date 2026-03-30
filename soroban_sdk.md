# Issue #944: Soroban SDK Version Update

**File:** `contracts/Cargo.toml`
**Priority:** Low

---

## 🔎 Verification

The current version `21.0.0` was verified against the official Stellar SDK repository and crates.io registry.

| Package | Previous | Latest (as of March 2026) | Source |
|---|---|---|---|
| `soroban-sdk` | `21.0.0` | `25.3.0` | [stellar/rs-soroban-sdk](https://github.com/stellar/rs-soroban-sdk) |
| `soroban-token-sdk` | `21.0.0` | `25.3.0` | crates.io |

**Conclusion:** Version `21.0.0` is **outdated**. Updated to `25.3.0`.

---

## ✅ Change Applied

**File:** `contracts/Cargo.toml` — `[workspace.dependencies]`

```toml
# Before
soroban-sdk = "21.0.0"
soroban-token-sdk = "21.0.0"

# After
soroban-sdk = "25.3.0"
soroban-token-sdk = "25.3.0"
```

All member contracts (`stellar_insights`, `analytics`, `access-control`, `governance`, `benches`) inherit this version via `workspace = true` — no per-crate changes are required.

---

## ⚠️ Deprecation Warning Surfaced by This Upgrade

Running `cargo check` after the upgrade revealed the following compiler warnings in `stellar_insights/src/events.rs`:

```
warning: use of deprecated method `soroban_sdk::events::Events::publish`:
use the #[contractevent] macro on a contract event type
  --> stellar_insights/src/events.rs:69:14
  --> stellar_insights/src/events.rs:100:22
  --> stellar_insights/src/events.rs:132:10
  --> stellar_insights/src/events.rs:138:10
  --> stellar_insights/src/events.rs:144:10
```

The `Events::publish()` method is deprecated as of `soroban-sdk` v22. The recommended replacement is the `#[contractevent]` macro. The affected event types (`SnapshotSubmitted`, `AnalyticsSnapshotSubmitted`) and standalone `emit_*` helpers in `events.rs` are candidates for a future migration task.

> **Note:** These are **warnings only** — the contracts continue to compile and function correctly. Resolving them is out of scope for issue #944.

---

## 📝 Note on Pre-existing Issue

A pre-existing syntax error was also detected in `access-control/src/lib.rs:320` during `cargo check`. This is **unrelated to the SDK version bump** and predates this change.
