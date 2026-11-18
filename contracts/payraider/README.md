# payraider

Soroban contract that anchors SHA-256 hashes of analytics snapshots on-chain,
one per epoch, so anyone can later verify a snapshot the backend served. The
backend calls it via `SNAPSHOT_CONTRACT_ID`.

## Who can anchor snapshots (#2600)

`submit_snapshot` is **deny-by-default**. It accepts a call only from:

- an address in the **authorised submitter set**, or
- an address holding `Role::Admin`.

Every other caller gets `Error::Unauthorized` (#3), even if it signed the call.

The submitter set is admin-managed and capped at `MAX_SUBMITTERS` (10):

| Entrypoint | Auth | Effect |
|---|---|---|
| `add_submitter(caller, submitter)` | admin | Adds a key |
| `remove_submitter(caller, submitter)` | admin | Removes a key |
| `rotate_submitter(caller, old, new)` | admin **and** `new` | Swaps `old` for `new` in one transaction |
| `rotate_submitter_with_policy(caller, old, new, overlap_ledgers, reason)` | admin **and** `new` | Swap with an optional overlap window (#2786) |
| `get_outgoing_submitters()` | none | Rotated-out keys still in their window, with expiry ledger |
| `get_submitters()` | none | Lists the set (the admin is not listed) |

`rotate_submitter` revokes the old key in the same transaction that authorises
the new one, so there's never a moment when both keys, or neither, can submit.
The new key must sign too, which proves it is controlled before it's trusted.
All three management calls work while the contract is paused, so a leaked key
can be replaced during an incident.

### Submitter rotation policy (#2786)

| Rule | Enforced by |
|---|---|
| Only the admin rotates; the **new** key must sign | `require_admin`, `new.require_auth()` |
| The **old** key's consent is not needed — it may be lost or leaked | by design |
| `new` must not be `old`, a current submitter, an outgoing key still in its window, or the admin / an `Admin` holder | `InvalidRotationTarget` (#33), `SubmitterAlreadyAuthorized` (#28) |
| Overlap: `old` keeps submitting until ledger `now + overlap_ledgers` (exclusive), at most `MAX_ROTATION_OVERLAP` (17,280 ≈ 1 day) | `InvalidOverlap` (#34) |
| `RotationReason::Compromise` ⇒ overlap must be `0` | `InvalidOverlap` (#34) |
| `remove_submitter(old)` ends an overlap window early | `delete_submitter` |
| `rotate_submitter` = no overlap, reason `Scheduled` | wrapper |

**Pending epochs.** Soroban transactions are atomic, so there is no half-written
epoch to clean up. A transaction signed by the old key that lands after its
window closes fails with `Unauthorized`; the backend resubmits the same epoch
with the new key, and epoch monotonicity is unaffected. Use an overlap window
for planned rotations so in-flight submissions still land. Operator steps:
[`docs/runbooks/submitter-key-rotation.md`](../../docs/runbooks/submitter-key-rotation.md).

`grant_role` / `revoke_role` with `Role::SnapshotSubmitter` add to / remove from
the same set, and `has_role(addr, SnapshotSubmitter)` reads from it. The set is
the only source of truth: per-address `SnapshotSubmitter` role entries written
by earlier builds are ignored.

### Migrating an existing deployment

After upgrading, add the backend's signing key (the account behind
`STELLAR_SOURCE_SECRET_KEY`) with `add_submitter` before it next submits.
Without that it can't anchor unless it's also the admin. Any key that was
granted `SnapshotSubmitter` under an earlier build has to be re-added the same
way, which is a chance to drop keys nobody can account for.

## Snapshot storage layout (storage v2, #2781)

Storage v1 kept every snapshot in one persistent `DataKey::Snapshots` map. Each
submission read and re-wrote the whole map, so its fee grew linearly with
history and the entry would eventually exceed the ledger entry size limit.

**Footprint projection (v1).** One map item is an `ScMap` key (u64, ~12 B) plus a
3-field struct (32 B hash + 2×u64 + 3 symbol keys, ~110 B XDR): ≈ 120 B per
epoch. Hourly epochs add 8,760 items ≈ **1.05 MB a year**, far past the
64 KiB per-entry ledger limit after ~550 epochs (about 3 weeks), after which
every submission fails.

**v2.** Each epoch is its own persistent entry `DataKey::Snapshot(epoch)` (~250 B
with chain fields), and the tip — `Latest`, `ChainHead`, `LatestEpoch` — lives
in instance storage. A submission touches one new persistent entry plus the
instance, so its cost is independent of history length
(`submission_cost_is_flat_in_history_length`, and the
`submit_snapshot_after_history` Criterion group in `contracts/benches`).

### Migrating a v1 deployment

```text
upgrade(new_wasm_hash)
migrate(admin)            -> 1   (moved 100, more remain; submissions refused)
migrate(admin)            -> 1
...
migrate(admin)            -> 2   (legacy map deleted, version stamped)
```

`migrate` moves at most `DEFAULT_MIGRATION_BATCH` (100) epochs per call;
`migrate_batch(caller, max_items)` takes 1..=`MAX_MIGRATION_BATCH` (500).
Epochs are moved in ascending order and removed from the legacy map in the same
transaction, so a failed or repeated call never duplicates or skips one.
`pending_migration()` reports what is left. Until it reaches 0,
`submit_snapshot` returns `MigrationRequired` (#26) so no new epoch can chain
onto an incomplete history. Each partial call emits `migr_part` with
`(caller, migrated, remaining)`.

## Snapshot hash chain (#2782)

Every stored epoch carries

```text
chain_hash = sha256(prev_chain_hash || epoch as u64 big-endian || snapshot_hash)
```

with `prev_chain_hash = 0x00…00` for the first stored epoch (the migration
chains legacy epochs the same way, oldest first). `prev_epoch` and
`prev_chain_hash` are stored alongside, and `chain_hash` is included in the
`SNAP_SUB` event.

| Entrypoint | Effect |
|---|---|
| `get_snapshot_record(epoch)` | Full `Snapshot` including chain fields |
| `get_chain_head()` | `(latest_epoch, chain_hash)` |
| `verify_range(from, to)` | Recompute every link from `to` back to the first epoch `>= from` (≤ `MAX_VERIFY_STEPS`); returns `ChainBroken` (#31) on any mismatch |

The on-chain view is a convenience. Because an upgrade can change the code, the
trust anchor is off-chain: keep the latest `chain_hash` you have seen (from
events or `get_chain_head`) and verify history against it with
`backend/src/services/snapshot_chain.rs` (`verify_full_history`). Rewriting any
past epoch changes every later `chain_hash`, so it can't match a head you
already recorded.

## Storage TTL (#2599, #2781)

| Data | Storage | Kept alive by |
|---|---|---|
| Per-epoch snapshot entry | persistent | its own `submit_snapshot`, `get_snapshot(_record)`, `bump_snapshot_ttl` |
| Latest snapshot entry | persistent | also `latest_snapshot`, `bump_ttl` |
| Admin, latest snapshot + chain head, submitter set, pause flag | instance | every state-changing call, `bump_ttl` |
| Admin role entry | persistent | admin `submit_snapshot`, `bump_ttl` |

Older epochs are allowed to expire: the chain head in instance storage commits
to all of them, so history stays verifiable from archived copies.
`bump_snapshot_ttl(from, to)` (permissionless, span ≤ `MAX_TTL_BUMP_SPAN` = 200)
keeps a range live if you want it readable on-chain.

### Restoring an archived epoch

An expired persistent entry is archived, not deleted. Restore it with a
`RestoreFootprint` operation, then read it as usual:

```bash
stellar contract restore --id "$SNAPSHOT_CONTRACT_ID" \
  --key-xdr "$(stellar xdr encode --type ScVal '{"vec":[{"symbol":"Snapshot"},{"u64":"<epoch>"}]}')" \
  --source "$KEEPER" --network mainnet
stellar contract invoke --id "$SNAPSHOT_CONTRACT_ID" -- get_snapshot_record --epoch <epoch>
```

Then `verify_range` or the backend verifier confirms it still links into the
current chain head.

Each extension pushes the entry about 30 days out (`LEDGERS_TO_EXTEND`, 518,400
ledgers at 5 s/ledger).

`get_snapshot` and `latest_snapshot` only extend TTLs when they're submitted as
transactions. A simulated read changes nothing on-chain. When there are no
submissions for a while, call **`bump_ttl()`** at least every few weeks. It's
permissionless, needs no auth, changes nothing except TTLs and emits no event.

## Errors added

| Code | Variant | When |
|---|---|---|
| 28 | `SubmitterAlreadyAuthorized` | `add_submitter` / `rotate_submitter` with a key already in the set |
| 29 | `SubmitterNotFound` | `remove_submitter` / `rotate_submitter` with a key not in the set |
| 30 | `TooManySubmitters` | Adding past `MAX_SUBMITTERS` |
| 31 | `ChainBroken` | `verify_range` found a link that does not recompute |
| 32 | `InvalidRange` | Bad `verify_range` / `bump_snapshot_ttl` range or `migrate_batch` size |
| 33 | `InvalidRotationTarget` | Rotating to the old key, an admin, or a key in its overlap window |
| 34 | `InvalidOverlap` | Overlap above `MAX_ROTATION_OVERLAP`, or any overlap on a compromise |
| 35 | `AdminRoleNotGrantable` | `grant_role(Admin)`, or `revoke_role(Admin)` on the current admin — use `set_admin` |

## Events added

All carry the second topic `SUBM_SET`.

| Topic 0 | Data | Emitted by |
|---|---|---|
| `subm_add` | `(admin, submitter)` | `add_submitter`, `grant_role(SnapshotSubmitter)` |
| `subm_rm` | `(admin, submitter)` | `remove_submitter`, `revoke_role(SnapshotSubmitter)` |
| `subm_rot` | `(admin, old_submitter, new_submitter)` | `rotate_submitter(_with_policy)` |
| `rot_pol` | `SubmitterRotationPolicy { admin, old_submitter, new_submitter, reason, overlap_until }` | `rotate_submitter(_with_policy)` |

`SNAP_SUB` data gains a `chain_hash` field (#2782). `migr_part` (no second
topic) carries `(caller, migrated, remaining)` for partial migrations.

## RBAC invariants (#2785)

`contracts/tests/fuzz/rbac_invariants.rs` drives random call sequences against
a reference model and checks after every step: exactly one admin; a revoked
submitter can never submit; while paused, snapshot state never changes; and
every call's outcome matches the model. Found a failure? Its seed is printed —
add it to `REGRESSION_SEEDS`. Budget is set with `RBAC_FUZZ_CASES` /
`RBAC_FUZZ_STEPS`; CI runs a larger, time-boxed pass in
`.github/workflows/contract-fuzzing.yml`.

To keep the single-admin invariant, `Role::Admin` now only moves through
`set_admin` / `set_admin_by_governance`.

## Running tests and benchmarks locally

```bash
cd contracts
cargo test -p payraider
cargo clippy -p payraider --all-targets -- -D warnings

# Submitter-set and TTL tests only
cargo test -p payraider submitter
cargo test -p payraider ttl

# Gas per entrypoint (see docs/GAS_COSTS.md)
cargo bench -p contract-benches --bench gas_benchmarks
```

The TTL tests advance the ledger sequence in the test environment and make no
network calls.
