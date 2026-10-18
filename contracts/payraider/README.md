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
| `get_submitters()` | none | Lists the set (the admin is not listed) |

`rotate_submitter` revokes the old key in the same transaction that authorises
the new one, so there's never a moment when both keys, or neither, can submit.
The new key must sign too, which proves it is controlled before it's trusted.
All three management calls work while the contract is paused, so a leaked key
can be replaced during an incident.

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

## Storage TTL (#2599)

| Data | Storage | Kept alive by |
|---|---|---|
| Snapshot map | persistent | `submit_snapshot`, `get_snapshot`, `latest_snapshot`, `bump_ttl` |
| Admin, latest epoch, submitter set, pause flag | instance | every state-changing call, `bump_ttl` |
| Admin role entry | persistent | admin `submit_snapshot`, `bump_ttl` |

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

## Events added

All carry the second topic `SUBM_SET`.

| Topic 0 | Data | Emitted by |
|---|---|---|
| `subm_add` | `(admin, submitter)` | `add_submitter`, `grant_role(SnapshotSubmitter)` |
| `subm_rm` | `(admin, submitter)` | `remove_submitter`, `revoke_role(SnapshotSubmitter)` |
| `subm_rot` | `(admin, old_submitter, new_submitter)` | `rotate_submitter` |

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
