# Design note: trust-minimised snapshot verification portal

Tracking issue: #2857
Status: proposed

## Problem

Each analytics snapshot is hashed and the SHA-256 is written on-chain. The point is that
anyone can check a published snapshot against the chain. In practice, verification goes
through our backend: it fetches the snapshot from our DB, recomputes the hash and tells
you whether it matches. A user who wants to catch us tampering has to trust the party
they are checking.

Goal: a third party can verify any epoch using only

1. the payload bytes, fetched from public content-addressed storage,
2. a hash function running on their own machine, and
3. a contract read sent to an RPC endpoint they choose.

None of these steps touches our API.

## Current state

Code checked on `main` at `d60acbb2`.

- [`services/snapshot.rs`](../../backend/src/services/snapshot.rs):
  `serialize_deterministically` produces canonical JSON (BTreeMap key order, arrays
  sorted by `normalize()`, RFC 3339 timestamps, ryu floats, and non-finite floats as the
  strings `"NaN"`, `"Infinity"` and `"-Infinity"`). `compute_sha256_hash_bytes` hashes the
  UTF-8 bytes. `store_snapshot_in_database` stores `canonical_json` in the `data` column.
- [`services/contract.rs`](../../backend/src/services/contract.rs):
  `submit_snapshot(hash, epoch)` invokes `submit_snapshot` on `SNAPSHOT_CONTRACT_ID`.
  `verify_snapshot_exists` ignores its arguments (`_hash`, `_ledger`) and is a stub.
- [`contracts/analytics`](../../contracts/analytics/src/lib.rs):
  `get_snapshot(epoch) -> Option<SnapshotMetadata { epoch, timestamp, hash: BytesN<32>, submitter, ledger_sequence, expires_at }>`.
  Snapshots can be pruned (`prune_snapshot_hash`) or expire (`submit_snapshot_with_ttl`).
- [`services/contract_listener.rs`](../../backend/src/services/contract_listener.rs)
  `verify_snapshot_with_backend`: the listener checks on-chain hashes by calling our own
  backend. This is the trust dependency the issue describes.
- The frontend has no verification page under `frontend/src/app/[locale]`.

## Design

### Overview

```
 backend (publisher)                         browser / CLI (verifier)
 ───────────────────                         ────────────────────────
 canonical_json ──sha256──► hash ──► chain ◄──── simulate get_snapshot(epoch) via user's RPC
       │                                                   │ on-chain hash
       ├──► IPFS (CID)  ┐                                  ▼
       └──► S3 /snapshots/<sha256>.json ──fetch bytes──► sha256(bytes) ── compare ──► match / mismatch
                        │
       manifest.json ◄──┘ (epoch → sha256, cid, url)  ── used only to locate bytes; never trusted for the result
```

The key point is that **the verifier hashes the exact bytes it downloads and never
re-serialises them.** JS and Rust format some floats differently (for example `1e21`
compared with `1000000000000000000000`), so parsing and re-serialising in the browser
would cause false mismatches.

### Publishing (backend)

New module `services/snapshot_publisher.rs`, called from `generate_and_submit_snapshot`
after the DB write succeeds. If publishing fails, the on-chain submission still goes
ahead and the publish is retried later (see failure modes).

```rust
#[async_trait]
pub trait SnapshotStore {
    /// Store bytes under their content address. Must be idempotent.
    async fn put(&self, sha256_hex: &str, bytes: &[u8]) -> Result<PublishedLocation>;
}
pub struct PublishedLocation { pub url: String, pub cid: Option<String> }
```

Implementations:

- `S3SnapshotStore` writes to `s3://$SNAPSHOT_BUCKET/snapshots/<sha256>.json` with
  `Content-Type: application/json`, `Cache-Control: public, max-age=31536000, immutable`
  and public-read. Because the key is the hash, an object cannot be swapped without
  changing its address.
- `IpfsSnapshotStore` (optional, behind `SNAPSHOT_IPFS_API_URL`) pins the bytes with
  raw leaves, so the CID is a direct function of the bytes.
- `LocalSnapshotStore` writes to a directory. Used for tests and local development.

Then it appends to a public **manifest** at `snapshots/manifest.json`:

```json
{
  "version": 1,
  "network": "testnet",
  "contract_id": "C...",
  "entries": [
    { "epoch": 42, "sha256": "ab12…", "url": "https://…/snapshots/ab12….json",
      "cid": "bafk…", "published_at": "2026-11-20T00:00:00Z" }
  ]
}
```

The manifest only tells the verifier where to find the bytes. Its `sha256` field is a
hint and is never used to decide the result. A tampered manifest can only cause a
mismatch or a "payload not found", never a false match.

Data model: migration `052_add_snapshot_publication.sql` adds `published_url`,
`published_cid`, `published_at` and `publish_attempts` to `snapshots`.

### Verifier library (browser and Node)

A new pure module, `sdk/typescript/src/verify/snapshot.ts`, re-exported for the frontend.
It has no dependency on our API client:

```ts
export interface VerifyOptions {
  rpcUrl: string;          // user-chosen; default from a hard-coded list per network
  contractId: string;      // pinned in the build; overridable by the user
  networkPassphrase: string;
  fetchPayload: (epoch: number) => Promise<Uint8Array>;  // gateway/S3/user file
}

export type EpochResult =
  | { epoch: number; status: "match"; onChainHash: string; payloadHash: string; ledger: number }
  | { epoch: number; status: "mismatch"; onChainHash: string; payloadHash: string; ledger: number }
  | { epoch: number; status: "not_on_chain"; reason: "missing" | "pruned" | "expired" }
  | { epoch: number; status: "payload_unavailable"; error: string }
  | { epoch: number; status: "rpc_error"; error: string };

export async function verifyEpoch(epoch: number, opts: VerifyOptions): Promise<EpochResult>;
```

Implementation:

1. **Hash.** `crypto.subtle.digest("SHA-256", bytes)` in the browser, or `node:crypto` in
   Node. No WASM is needed because WebCrypto runs locally and SHA-256 is all we need.
2. **Chain read.** `@stellar/stellar-sdk` (already a frontend dependency, version 17.2.1):
   build a `get_snapshot(epoch)` invocation, call `rpc.Server.simulateTransaction`, and
   decode the `Option<SnapshotMetadata>` ScVal. `None` gives `not_on_chain`. An
   `expires_at` in the past gives `expired`. Simulation needs no signing and no funded
   account. It uses a throwaway source account with sequence 0.
3. **Compare** in constant time, as lowercase hex.
4. **Sanity check.** Parse the payload and confirm `payload.epoch === epoch`. If it does
   not match, return `mismatch`. This stops a valid payload for epoch A from being
   presented as epoch B.

The same module backs a CLI (`npx @stellar-insights/sdk verify-snapshot --epoch 42 --rpc …`)
for people who do not want to trust JavaScript served from our domain.

### Portal UI

Route: `frontend/src/app/[locale]/verify/page.tsx`. The page is client-only
(`"use client"`), and every network call goes directly from the browser to the RPC or the
storage gateway.

- Inputs: epoch range, RPC URL (editable, defaulting to the public SDF RPC for the
  network), contract ID (shown in full with a copy button and a link to a block
  explorer), and payload source (the manifest URL, an IPFS gateway, or file upload).
- Output: a table with one row per epoch (epoch, ledger, on-chain hash, payload hash, and
  a status badge). Mismatches are listed first.
- An "Advanced" panel shows the exact RPC request and the raw bytes' hash, so a user can
  reproduce the check by hand.
- Strings go through `next-intl` like the rest of `[locale]`.
- No call to `NEXT_PUBLIC_API_URL` is allowed from this route. A unit test enforces this
  (see test plan).

### Trust boundary

| Component | Trusted? | Why |
|---|---|---|
| Our API | No | Never called |
| Manifest | No | Only used to locate bytes |
| Storage (S3/IPFS) | No | Content-addressed; tampering produces a mismatch |
| RPC endpoint | User's choice | A dishonest RPC could lie about the chain. Users can choose their own, and the CLI accepts several RPCs and requires them to agree |
| Contract ID | Pinned in build and docs | Shown in the UI. Changing it is a reviewed code change and is announced |
| Verifier JS served by us | Partially | Mitigated by the open-source CLI, which gives the same result off-site |

## Failure modes

| Failure | Behaviour |
|---|---|
| Publish to S3/IPFS fails | The snapshot is still submitted on-chain. A `publish_attempts` retry job runs with backoff. `snapshot_publish_failures_total` increments. The epoch shows `payload_unavailable` until it is published |
| Payload not found at the gateway | `payload_unavailable`, with the URL that was tried. The user can upload a file instead |
| Epoch pruned or expired on-chain | `not_on_chain` with a reason. Reported separately from `mismatch` |
| RPC down or rate-limited | `rpc_error` for that epoch only. Range verification continues, with concurrency capped at 4 |
| Payload bytes altered (whitespace, re-encoding, a changed value) | `mismatch` |
| Payload is valid but for a different epoch | `mismatch` (epoch sanity check) |
| Large range | Verified in batches. Progress is shown. The user can cancel |

## Test plan

Library (`sdk/typescript`, vitest), with a mocked `simulateTransaction` and fixture bytes
taken from the Rust serializer:

1. **Main path.** Fixture payload and the on-chain hash it produces give `match`.
2. **Tampered payload is detected** (this is the issue's test plan). Change one digit,
   add one trailing space, or swap two keys. Each gives `mismatch`.
3. **Epoch swap.** A valid payload for epoch 41 checked as epoch 42 gives `mismatch`.
4. **Not on chain.** `None` from the contract gives `not_on_chain: missing`, and a past
   `expires_at` gives `expired`.
5. **Cross-language fixture.** A Rust test writes `canonical_json` and its hash to
   `sdk/typescript/test/fixtures/snapshot_epoch_*.json`. The TS test hashes the bytes and
   must get the same hash. This catches serializer drift between the publisher and the
   verifier.
6. **Floats that JS formats differently** (`1e21`, `5e-324`, `"NaN"`): the bytes still
   hash to the expected value. This is a regression test for the no-re-serialise rule.

Frontend (vitest + Testing Library):

7. The page renders a row per epoch with the right status badge.
8. **No calls to our API.** Spy on `fetch` and assert that no request goes to
   `NEXT_PUBLIC_API_URL`.

Backend (`cargo test`):

9. `LocalSnapshotStore` round-trip. The stored bytes equal `canonical_json` exactly and
   the key equals its SHA-256.
10. A failed publish does not block submission, and the retry picks it up.

## Docs to update

- New user-facing page: `docs/SNAPSHOT_VERIFICATION.md`, covering how to verify in the
  browser, with the CLI, and by hand with `sha256sum` plus `stellar contract invoke`.
- [INTEGRATION_GUIDE.md](../INTEGRATION_GUIDE.md): link to the verifier SDK function.
- `.env.example`: `SNAPSHOT_BUCKET`, `SNAPSHOT_PUBLIC_BASE_URL`, `SNAPSHOT_IPFS_API_URL`.

## Deferred

- Removing `verify_snapshot_with_backend` from the listener. It can switch to the same
  hash-the-bytes logic once publishing lands, in a follow-up.
- Backfilling publication for historical epochs. A one-off admin task.
- Verifying the snapshot's *contents* (recomputing metrics from raw ledger data). This
  design proves integrity (we did not change the data after committing to it), not
  correctness (the metrics were computed right).
- Signed manifests. They are not needed for integrity, but would help with discovery.
