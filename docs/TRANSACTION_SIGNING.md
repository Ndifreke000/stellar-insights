# Transaction signing

Soroban snapshot submissions are signed by a **submitter key** — the Stellar
account authorized in the contract's submitter set. Whatever that key signs
gets anchored on-chain, so the private key should never sit in the web
process's memory in production. This document describes the signer
abstraction that makes that possible.

Code lives in `backend/src/services/`:

- `transaction_signer.rs` — the `TransactionSigner` trait and backends
- `envelope_policy.rs` — pre-signature validation of the simulated envelope
- `signature_audit.rs` — audit trail for every signature attempt
- `contract.rs` — wires it all into `ContractService`

## Design

### `TransactionSigner` trait

```rust
#[async_trait]
pub trait TransactionSigner: Send + Sync {
    fn backend(&self) -> &'static str;   // "local-seed" | "aws-kms" | "gcp-kms"
    fn key_id(&self) -> &str;             // ARN / key-version name / env var
    async fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;   // 64-byte Ed25519 sig
    async fn public_key(&self) -> Result<[u8; 32]>;            // raw ed25519 pubkey
}
```

The signer only ever receives the **32-byte transaction signature base**
(`SHA256(NETWORK_ID ++ XDR(TransactionSignaturePayload))`). It cannot see,
let alone alter, the transaction itself.

### Backends

| Backend | `TX_SIGNER_BACKEND` | Key location | Notes |
|---|---|---|---|
| `LocalSeedSigner` | `local` (default) | `STELLAR_SOURCE_SECRET_KEY` env var, in memory | Dev/test only |
| `GcpKmsSigner` | `gcp-kms` | Cloud KMS `EC_SIGN_ED25519` key version | REST API via `reqwest`; auth via `GCP_ACCESS_TOKEN` or a service-account JWT (`GOOGLE_APPLICATION_CREDENTIALS`) |
| `AwsKmsSigner` | `aws-kms` | AWS KMS `ECC_NIST_EDWARDS25519` key (`ED25519_SHA_512`, `MessageType::RAW`) | Requires `cargo build --features aws-kms` |

Both KMS signers fetch the public key eagerly at startup, so a missing IAM
permission or wrong key spec fails fast instead of at first submission.
After every sign, the backend verifies the returned signature against that
public key before attaching it to the envelope — a misbehaving KMS cannot
produce a transaction that fails on-chain.

### Envelope policy

`prepare_and_sign_transaction` decodes the `transactionData` XDR returned by
`simulateTransaction` and runs `validate_submit_snapshot_envelope` **before**
calling the signer. The envelope must be exactly:

- one operation: `InvokeHostFunction` → `InvokeContract`
- `contract_address` == `SNAPSHOT_CONTRACT_ID`
- `function_name` == `submit_snapshot`
- args == `[bytes(hash), u64(epoch)]`, matching `build_invoke_args`

Anything else — wrong contract, wrong function, extra operations, tampered
args — is rejected and audited as `policy_rejected`; the KMS is never called.
This keeps the submitter key from becoming a signing oracle if the RPC
endpoint or simulation response is ever compromised.

### Audit trail

Every attempt emits a structured `tracing` event on the `signature_audit`
target (backend, key id, tx-hash, contract, function, epoch, status,
latency). When a pool is attached via
`ContractService::with_audit_pool(db.pool().clone())`, each attempt is also
persisted to the `transaction_signature_audit` table (migration 059) with
status `signed` / `policy_rejected` / `sign_error`.

Audit writes are best-effort: a DB failure is logged but never blocks
anchoring; the tracing event is always emitted.

## Failure modes

| Failure | Behaviour |
|---|---|
| Simulation returns malformed/foreign XDR | Policy rejects before signing; audited |
| Signer returns wrong-length or invalid signature | Rejected by local verify; audited `sign_error` |
| KMS unreachable / IAM denied | `sign_error` audit; retry loop in `submit_snapshot_hash` applies |
| No signer configured (read-only client) | Explicit error; nothing is signed |
| Audit DB insert fails | `tracing::error!`; submission proceeds |

## Configuration

See `backend/.env.example` ("Transaction signer backend"). Minimum for
production with GCP KMS:

```bash
TX_SIGNER_BACKEND=gcp-kms
GCP_KMS_KEY_VERSION_NAME=projects/<p>/locations/<l>/keyRings/<r>/cryptoKeys/<k>/cryptoKeyVersions/1
GOOGLE_APPLICATION_CREDENTIALS=/run/secrets/gcp-sa.json
SNAPSHOT_CONTRACT_ID=<C...>
# STELLAR_SOURCE_SECRET_KEY is NOT required and is ignored by KMS backends
```

For AWS KMS, build with `--features aws-kms` and set `AWS_KMS_KEY_ID` plus
the usual AWS credential chain (`AWS_REGION`, env keys, shared config, or
instance role). `AWS_ENDPOINT_URL` can point at localstack for testing.

## Tests

- `transaction_signer.rs`: local signer sign/verify roundtrip, hint
  derivation, bad-seed rejection, public-key decoding helpers.
- `envelope_policy.rs`: accepts the exact invocation; rejects wrong
  function, contract, hash, epoch, missing/extra ops, missing args, bad
  config id.
- `signature_audit.rs`: event construction, tracing-only path, DB insert.
- KMS signers are exercised against localstack/mock KMS endpoints in CI
  where available; the trait is backend-agnostic so conformance is covered
  by the local signer tests.
