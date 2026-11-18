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

## Admin transfer is two-step (#2783)

`set_admin` is gone. No single transaction can hand the contract to a new key:

| Entrypoint | Auth | Effect |
|---|---|---|
| `propose_admin(caller, new_admin)` | admin | Records `new_admin` as pending. Nothing else changes. |
| `accept_admin(new_admin)` | `new_admin` | Completes the transfer if `new_admin` is the pending address |
| `cancel_admin_transfer(caller)` | admin or governance | Drops the pending proposal |
| `get_pending_admin()` | none | The pending address, if any |

A typo'd address can never sign `accept_admin`, so the worst a mistake costs is
a cancelled proposal. `set_admin_by_governance` still transfers immediately
(the vote is the delay) and clears any pending proposal.

## Upgrades are timelocked (#2783)

`upgrade` is replaced by a two-phase flow, so observers see new code coming
before it runs:

| Entrypoint | Auth | Effect |
|---|---|---|
| `schedule_upgrade(caller, wasm_hash)` | admin, `UpgradeManager` or governance | Records a `PendingUpgrade { wasm_hash, proposer, scheduled_at, executable_at }` |
| `execute_upgrade(caller)` | admin, `UpgradeManager` or governance | Installs the Wasm once `ledger >= executable_at` |
| `cancel_upgrade(caller)` | admin or governance | Drops the scheduled upgrade |
| `get_pending_upgrade()` | none | The scheduled upgrade, if any |
| `get_upgrade_delay()` / `set_upgrade_delay(caller, ledgers)` | none / admin or governance | Delay in ledgers |

The delay defaults to `DEFAULT_UPGRADE_DELAY_LEDGERS` (17,280, ~24 h) and can
never go below `MIN_UPGRADE_DELAY_LEDGERS` (720, ~1 h). The admin can only
lengthen it; only governance can shorten it, so a stolen admin key can't
shrink the window and upgrade at once. Scheduling and executing are refused
while paused. Run `migrate()` after `execute_upgrade` if the schema changed.

Observers can watch for pending changes with `get_pending_admin()` /
`get_pending_upgrade()` or by indexing the events below.

## Storage TTL (#2599, #2784)

Every key and its TTL policy:

| Key | Storage | Kept alive by |
|---|---|---|
| `Snapshots` (snapshot map) | persistent | `submit_snapshot`, `get_snapshot`, `latest_snapshot`, `bump_ttl` |
| `Roles(addr)` (any address, not just the admin) | persistent | every role check on that address (`has_role`, `pause`, `unpause`, `submit_snapshot` by an admin, upgrade auth), `grant_role` / `revoke_role`, `bump_ttl` |
| `Admin`, `LatestEpoch`, `Paused`, `Version`, `Governance`, `StorageVersion`, `Submitters`, `PendingAdmin`, `PendingUpgrade`, `UpgradeDelay`, `RoleHolders` | instance | every state-changing call, `bump_ttl` |

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
ledgers at 5 s/ledger). The submitter set is in instance storage, so it lives
exactly as long as the contract instance.

`bump_ttl` finds role entries through the `RoleHolders` index (instance
storage, capped at `MAX_ROLE_HOLDERS` = 32). `grant_role` adds to it,
`revoke_role` removes the address once it holds no roles (the entry is deleted
rather than left as an empty list), and admin transfers move it. Addresses
granted roles before this build aren't in the index until they're granted a
role again — re-grant them once after upgrading. The admin is always bumped.

`get_snapshot` and `latest_snapshot` only extend TTLs when they're submitted as
transactions. A simulated read changes nothing on-chain. When there are no submissions for a
while, call **`bump_ttl()`** at least every few weeks. It's permissionless, needs no auth, changes nothing
except TTLs and emits no event.

### Restoring an archived entry

If an entry's TTL does run out it is **archived**, not deleted: reads of it
fail (a role check on an archived `Roles(addr)` entry aborts the call rather
than returning `false`, see `archived_role_entry_cannot_be_read_without_restore`).
To bring it back:

1. Simulate the failing call; the RPC response includes a `restorePreamble`
   with the footprint of the archived keys.
2. Submit a `RestoreFootprint` operation with that footprint, e.g.
   `stellar contract restore --id <CONTRACT_ID> --key-xdr <KEY_XDR> --source <ACCOUNT>`
   (or `--durability persistent --key Snapshots` for the snapshot map).
3. Call `bump_ttl()` so the restored entry gets a full TTL, then retry.

## Errors added

| Code | Variant | When |
|---|---|---|
| 28 | `SubmitterAlreadyAuthorized` | `add_submitter` / `rotate_submitter` with a key already in the set |
| 29 | `SubmitterNotFound` | `remove_submitter` / `rotate_submitter` with a key not in the set |
| 30 | `TooManySubmitters` | Adding past `MAX_SUBMITTERS` |
| 31 | `NoPendingAdmin` | `accept_admin` / `cancel_admin_transfer` with nothing proposed |
| 32 | `NoPendingUpgrade` | `execute_upgrade` / `cancel_upgrade` with nothing scheduled |
| 33 | `InvalidUpgradeDelay` | Delay below the minimum, or the admin shortening it |
| 34 | `TooManyRoleHolders` | Granting a role to a new address past `MAX_ROLE_HOLDERS` |

`execute_upgrade` before its delay returns the existing `TimelockNotExpired` (17).

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

Contract lifecycle events (second topic `CTR_LFE`, #2783):

| Topic 0 | Data | Emitted by |
|---|---|---|
| `adm_prop` | `(admin, proposed)` | `propose_admin` |
| `adm_cncl` | `(caller, proposed)` | `cancel_admin_transfer` |
| `admin` | `AdminTransferredEvent` | `accept_admin`, `set_admin_by_governance` |
| `upg_schd` | `(proposer, wasm_hash, executable_at)` | `schedule_upgrade` |
| `upg_cncl` | `(caller, wasm_hash)` | `cancel_upgrade` |
| `upg_exec` | `(caller, wasm_hash)` | `execute_upgrade` (the legacy `("upgrade",)` event is still emitted too) |
| `upg_dely` | `(caller, old_delay, new_delay)` | `set_upgrade_delay` |

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
