# Multisig: Transaction Coordination Service with Partial Signature Collection

**GitHub Issue**: [#2854](https://github.com/Ndifreke000/stellar-insights/issues/2854)
**Related**: [SNAPSHOT_ANCHORING.md](./SNAPSHOT_ANCHORING.md), [security/THREAT_MODEL.md](./security/THREAT_MODEL.md)

---

## Overview

Moving the snapshot contract's admin, and other privileged accounts, to a Stellar multisig account means several people must sign the same transaction. Each signs in their own wallet, at a different time. Someone has to hold the envelope, collect the signatures, tell people when their signature is needed, and submit once the threshold is met. That is this service.

**The backend never holds signing keys.** It verifies and assembles signatures; it does not produce them.

---

## Current State

More exists than the issue suggests. A basic coordination API is already live:

| File | What it does today |
|---|---|
| `backend/src/multisig.rs` | Pure functions: `transaction_hash` (network-bound signature payload hash), `source_account`, `extract_signature` (accepts a raw base64 Ed25519 sig **or** a wallet-signed envelope, verifies against the signer's key), `assemble` (attach `DecoratedSignature`s), `submit` (POST to Horizon). Unit-tested |
| `backend/src/api/transactions.rs` | `GET/POST /api/transactions`, `GET /{id}`, `POST /{id}/signatures`, `POST /{id}/submit`, behind SEP-10 wallet auth. Signature insert + "ready" promotion run in one DB transaction |
| `backend/migrations/011_create_transactions.sql` | `pending_transactions(id, source_account, xdr, required_signatures, status)`, `transaction_signatures(transaction_id, signer, signature, UNIQUE(transaction_id, signer))` |

### Gaps

| # | Severity | Gap |
|---|---|---|
| M1 | **High** | **No signer allowlist.** `add_signature` accepts a valid signature from *any* SEP-10 wallet (the only check is `req.signer == wallet`). Five random wallets can sign and push a transaction to `ready`. Horizon rejects it on submit, but `ready` is misleading, and the UI and notifications would act on it |
| M2 | High | **Thresholds are a signature count, not weights.** Stellar authorises by summing signer weights against the operation's threshold (low/med/high). `required_signatures` cannot represent "admin key weight 2 + any one of three ops keys weight 1, high threshold 3" |
| M3 | High | **Only the transaction's source account is considered.** Operations can have their own `source_account`, each needing that account's signers. The contract-admin transaction is often submitted from a fee-paying account while the operation's source is the admin multisig |
| M4 | Medium | **No expiry.** Transactions without `TimeBounds.max_time` stay `pending` forever. Transactions with time bounds are not tracked, so expired ones still show as signable |
| M5 | Medium | **Stale sequence numbers.** If the source account's sequence moves (any other transaction from it), the stored envelope becomes permanently invalid (`tx_bad_seq`), and nothing detects or reports it |
| M6 | Medium | **No notifications.** Signers learn a transaction exists only by polling |
| M7 | Low | `status` values in use (`pending`, `ready`, `submitted`, `failed`) don't match the migration comment, and there is no CHECK constraint |
| M8 | Low | Submit is creator-only, and a lost race (two submits) is handled only by the `submitted` check before the network call |

---

## Design

### 1. Data model

Keep the existing tables and extend them in a new migration:

```sql
ALTER TABLE pending_transactions ADD COLUMN network_passphrase TEXT;   -- pin at creation
ALTER TABLE pending_transactions ADD COLUMN tx_hash TEXT;              -- hex, computed at creation
ALTER TABLE pending_transactions ADD COLUMN title TEXT;                -- human description
ALTER TABLE pending_transactions ADD COLUMN created_by TEXT;           -- SEP-10 wallet
ALTER TABLE pending_transactions ADD COLUMN expires_at TEXT;           -- min(TimeBounds.max_time, created + max_ttl)
ALTER TABLE pending_transactions ADD COLUMN sequence_number TEXT;      -- from the envelope
ALTER TABLE pending_transactions ADD COLUMN submitted_hash TEXT;
ALTER TABLE pending_transactions ADD COLUMN last_error TEXT;
CREATE UNIQUE INDEX idx_pending_tx_hash ON pending_transactions(tx_hash);

-- One row per account whose authorisation the transaction needs (tx source + each distinct op source).
CREATE TABLE pending_transaction_requirements (
    transaction_id   TEXT NOT NULL REFERENCES pending_transactions(id) ON DELETE CASCADE,
    account          TEXT NOT NULL,          -- G...
    threshold_level  TEXT NOT NULL CHECK (threshold_level IN ('low','medium','high')),
    threshold        INTEGER NOT NULL,       -- snapshot of the account's threshold at that level
    PRIMARY KEY (transaction_id, account)
);

-- Snapshot of each required account's signers at creation (refreshed on demand).
CREATE TABLE pending_transaction_signers (
    transaction_id   TEXT NOT NULL REFERENCES pending_transactions(id) ON DELETE CASCADE,
    account          TEXT NOT NULL,          -- the account being authorised
    signer           TEXT NOT NULL,          -- G... ed25519 signer key
    weight           INTEGER NOT NULL,
    PRIMARY KEY (transaction_id, account, signer)
);
```

`required_signatures` stays for backwards compatibility, but it no longer decides anything (see §3).

Statuses become an explicit state machine:

```
          ┌──────────── expire ────────────┐
          │                                ▼
 pending ─┼─ threshold met ─► ready ─► submitting ─► submitted
          │                    │           │
          │                    └─ expire ──┼─────────► expired
          └─ seq stale ────────────────────┴─ seq stale/horizon error ─► failed | stale
 any non-terminal ── creator cancels ──► cancelled
```

Terminal states: `submitted`, `failed`, `expired`, `stale`, `cancelled`. Add `CHECK (status IN (...))`.

### 2. Creating a coordinated transaction

`POST /api/transactions` (existing route, extended request):

```json
{
  "xdr": "AAAAAgAAAA…",
  "title": "Rotate snapshot contract admin to multisig",
  "notify": true
}
```

On the server:
1. Decode the envelope (v1 only; fee-bump wrapping happens at submit, see §5). Compute `tx_hash` with the configured network passphrase. If that hash already exists, return `409` along with the existing ID.
2. Collect the required accounts: the tx `source_account` plus each operation's `source_account`, de-duplicated. For each one, load it from Horizon (`GET /accounts/{id}`) to get its `signers[]` and `thresholds`.
3. Work out the threshold level each account needs from its operations, using the [Stellar docs table](https://developers.stellar.org/docs/learn/encyclopedia/security/signatures-multisig#thresholds) (e.g. `SetOptions` changing signers is high, `Payment` is medium, `BumpSequence` is low). The tx source always needs at least low. Put this mapping in `multisig.rs` as `fn required_threshold(op: &OperationBody) -> ThresholdLevel`, with a unit test per operation type.
4. Persist the requirement and signer snapshots, `expires_at` and `sequence_number`.
5. Authorisation: the creator must be a signer of at least one required account. Without this check, anyone could spam signers with transactions to approve.

`required_signatures` from older clients is ignored. If it is present, the response carries a `Deprecation` header.

### 3. Adding a partial signature

`POST /api/transactions/{id}/signatures` (existing route, same body):

1. Reject anything in a terminal state, or past `expires_at` (`409 expired`).
2. **Allowlist (fixes M1)**: `req.signer` must appear in `pending_transaction_signers` for this transaction. If not, return `403 not_a_signer`.
3. Verify with the existing `multisig::extract_signature` against the **stored** `network_passphrase`, not the current env (the env could change between creation and signing).
4. Insert. The existing `UNIQUE(transaction_id, signer)` handles duplicates; return `409` instead of `400`.
5. **Recompute readiness (fixes M2, M3)**: the transaction is ready when, for **every** required account, the sum of weights of collected signers for that account is at least that account's threshold. One signer can count toward several accounts (e.g. a key that signs for both the fee source and the admin account).
6. Return the progress, so clients don't need a second GET:

```json
{
  "status": "pending",
  "requirements": [
    { "account": "GADMIN…", "level": "high", "threshold": 3, "collected_weight": 2,
      "signed_by": ["GKEY1…"], "awaiting": ["GKEY2…", "GKEY3…"] },
    { "account": "GFEE…",   "level": "low",  "threshold": 1, "collected_weight": 1,
      "signed_by": ["GFEE…"], "awaiting": [] }
  ],
  "expires_at": "2026-11-28T12:00:00Z"
}
```

`GET /api/transactions/{id}` returns the same `requirements` block, and adds a `signature_base` field (hex `tx_hash`). Hardware-wallet users who sign a raw hash need it.

Keep the whole flow in one DB transaction, as today. With SQLite, the `BEGIN` already serialises writers. On Postgres, add `SELECT … FOR UPDATE` on the `pending_transactions` row.

### 4. Expiry and sequence handling (fixes M4, M5)

**Expiry**
- `expires_at = min(envelope TimeBounds.max_time, created_at + MULTISIG_MAX_TTL)`. `MULTISIG_MAX_TTL` defaults to 7 days.
- If the envelope has no `max_time`, warn at creation (`"warnings": ["no_time_bounds"]`). An envelope without time bounds stays valid on-chain forever, so a signature collected today could be submitted by anyone years later. Creation is allowed, but the UI must surface the warning. Optional flag: `MULTISIG_REQUIRE_TIME_BOUNDS=true` makes it a hard `400`.
- A background job (`jobs/` pattern, every 60 s) moves non-terminal rows past `expires_at` to `expired` and notifies the signers.

**Sequence**
- The same job, for non-terminal rows, fetches the source account's current `sequence` from Horizon. Results are cached per account for 30 s, so N pending transactions on one account cost one request.
  - `account.sequence >= tx.seq_num`: some other transaction consumed the sequence, so this envelope can never succeed. Mark it `stale` and notify the creator with "re-create with a fresh sequence number".
  - `tx.seq_num > account.sequence + 1`: the envelope is not submittable yet; earlier transactions must land first. Expose this as `"blocked_by_sequence": true`, but leave the status alone.
- At submit, do the same check before calling Horizon, and map Horizon's `tx_bad_seq` to `stale`.
- **Signer drift**: if an account's signers or thresholds change after creation, the snapshot is wrong. Refresh the snapshot at submit time and recompute readiness. If the transaction is no longer ready, return `409 signers_changed` with the new requirements. Signatures from keys that were removed are kept but no longer count.

### 5. Submitting

`POST /api/transactions/{id}/submit`:
- Any **signer** may submit, not just the creator (fixes M8). That removes a single person as a bottleneck.
- Atomically claim the row with `UPDATE … SET status='submitting' WHERE id=? AND status='ready'`. If zero rows change, return `409`. This stops double submission.
- Assemble with `multisig::assemble`, then submit via `multisig::submit`.
  - Success: `submitted`, store `submitted_hash`, notify everyone.
  - `tx_bad_seq`: `stale`.
  - `tx_too_late`: `expired`.
  - `tx_bad_auth` / `tx_bad_auth_extra`: `failed`, with the result codes stored in `last_error`. Signer drift or a threshold bug is the likely cause, so log it at `error`.
  - Network error or timeout: go back to `ready`, then poll `GET /transactions/{tx_hash}` before allowing a retry, because the submission may have landed.
- Optional fee-bump: `{"fee_bump": {"fee_source": "G…", "max_fee": 10000}}` is out of scope here. It needs a second signature collection round for the fee source. Track it as a follow-up.

### 6. Notifications (fixes M6)

Signers are identified by Stellar account. To reach them, a signer links a contact channel to their wallet once, through a SEP-10-authenticated `POST /api/transactions/notification-preferences`:

```json
{ "email": "ops@example.com", "telegram_chat_id": "…", "webhook_url": "https://…", "events": ["signature_requested", "ready", "submitted", "expired", "stale"] }
```

Deliver through the services that already exist: `backend/src/email/`, `backend/src/telegram/`, and `services/webhook_dispatcher.rs` (which reuses the existing HMAC signing and SSRF protection, see [WEBHOOK_SSRF_PROTECTION.md](./security/WEBHOOK_SSRF_PROTECTION.md)). Also broadcast in-app over the existing WebSocket on topic `multisig:{account}`.

| Event | Recipients |
|---|---|
| `signature_requested` | Every signer in the snapshot except the creator |
| `signature_added` | Creator (digest, at most one per 10 minutes) |
| `ready` | All signers |
| `submitted` / `failed` / `stale` / `expired` | All signers |
| `expiring_soon` | Signers who haven't signed, 24 h before `expires_at` |

Notifications contain the transaction title, its ID, a deep link, and the `tx_hash`. **They never contain the XDR**, because signers must review it in the UI or their wallet and not trust the message. Store sends in an outbox table for idempotent retry.

### 7. UI (minimal)

Extend `frontend/src/app/[locale]/transactions/`:
- **Inbox**: transactions where the connected wallet is an awaiting signer.
- **Detail**: decoded operations (human-readable), requirements and progress bars, time remaining, `tx_hash`, a "Sign with wallet" button (Freighter `signTransaction`, then POST the returned envelope), and a "Submit" button when the transaction is ready.
- A prominent warning when there are no time bounds, or when the transaction is stale.

---

## Security Notes

- The XDR is untrusted input. Decode with the existing 1 MB limit, and reject envelopes with more than 100 operations.
- Display decoded operations **from the stored XDR**, never from client-supplied metadata such as `title`. The title is a label, not a description of what will execute.
- Pin `network_passphrase` per row. If testnet signatures are submitted against mainnet, the hash does not match and the signature is useless, which is safe, but it should be caught early with a clear error.
- Rate-limit creation per wallet (e.g. 20/hour) to prevent notification spam.
- Audit every state transition with the acting wallet (see [AUDIT_LOG.md](./AUDIT_LOG.md)).

---

## Test Plan

**Unit (`multisig.rs`)**
- `required_threshold` for every `OperationBody` variant.
- `required_accounts` collects the tx source plus distinct op sources, including muxed sources (they resolve to the underlying `G…`).
- `is_ready` with weights: below threshold, exactly at threshold, above it, one signer counting for two accounts, and weight-0 signers (they contribute nothing).

**Integration (mock Horizon with `wiremock`)**
- A signature from a wallet outside the snapshot gets `403` (regression for M1).
- 2-of-3 weighted: the first signature leaves the transaction `pending`, the second makes it `ready`.
- A transaction with an op source different from the tx source is not ready until both accounts are satisfied.
- A duplicate signature gets `409`.
- An expired transaction rejects signatures and submit; the job marks it `expired`.
- The account sequence advances, the job marks the transaction `stale`, and a notification is enqueued.
- Signers change on Horizon before submit: `409 signers_changed`.
- Two concurrent submits: exactly one calls Horizon.
- Horizon `tx_bad_seq` / `tx_too_late` / `tx_bad_auth` map to the right statuses.
- A Horizon timeout followed by the transaction appearing on-chain ends in `submitted`, not a double submit.

**End-to-end (testnet, `#[ignore]`, run manually)**
- Create a 2-of-3 account on testnet, coordinate a `SetOptions` transaction through the API, and confirm it lands.

---

## Acceptance Criteria

- [ ] Only signers of a required account can add signatures (M1 fixed, with a regression test).
- [ ] Readiness uses Stellar weights and thresholds across all required accounts.
- [ ] Expiry and stale-sequence detection move transactions to terminal states and notify signers.
- [ ] Submission is idempotent and safe under concurrency.
- [ ] Notifications are delivered by email, Telegram, webhook and in-app, without XDR in the payload.
- [ ] The existing API stays compatible for one release (`required_signatures` accepted but deprecated).
- [ ] This doc and the OpenAPI annotations in `api/transactions.rs` are updated.

## References

- [Stellar: Signatures and multisig](https://developers.stellar.org/docs/learn/encyclopedia/security/signatures-multisig)
- [Stellar: Transaction lifecycle, time bounds, sequence numbers](https://developers.stellar.org/docs/learn/fundamentals/transactions/transaction-lifecycle)
- [Horizon: Submit a transaction (result codes)](https://developers.stellar.org/docs/data/apis/horizon/api-reference/errors/result-codes/transactions)
