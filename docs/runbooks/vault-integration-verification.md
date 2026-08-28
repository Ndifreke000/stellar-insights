# Runbook: Vault Integration Verification (dev & prod credential modes)

Verification steps for `backend/src/vault/` — the `VaultClient`, dynamic database
credential generation, lease management, and KV v2 secret reads.

Related issue: [#1858](https://github.com/Ndifreke000/stellar-insights/issues/1858).

Fixing the rustdoc examples in `client.rs` made `cargo test --all-features` pass;
it did **not** prove the integration works against a live Vault. Run the checks
below before relying on the Vault path in a release.

## 1. Confirm the Vault workflows ran green

```bash
gh run list --workflow=vault-deploy.yml --limit 5
gh run list --workflow=vault-deploy-approle.yml --limit 5
```

Both must show a recent `success` on `main`. If the most recent run predates the
last change under `backend/src/vault/`, re-run them before shipping:

```bash
gh workflow run vault-deploy.yml
```

## 2. Credential modes

| Mode | Source of DB credentials | Config |
| --- | --- | --- |
| **Dev** | `DATABASE_URL` env var directly | Vault env vars unset |
| **Prod** | `VaultClient::get_database_credentials()` (dynamic, leased) | `VAULT_ADDR`, `VAULT_TOKEN`, `VAULT_NAMESPACE` (optional), DB role |

`VaultConfig::from_env()` returns `Err(VaultError)` when the Vault variables are
absent — it does not panic. Dev startup is expected to log that Vault is not
configured and fall back to `DATABASE_URL`.

**Verify the fallback is non-fatal:**

```bash
# Vault deliberately unset / unreachable
unset VAULT_ADDR VAULT_TOKEN
cargo run --bin stellar-insights-backend
# Expect: a warning about Vault being unconfigured, then a normal startup on /health
```

```bash
# Vault configured but unreachable — must degrade, not hang or crash
VAULT_ADDR=http://127.0.0.1:1 VAULT_TOKEN=dummy cargo run --bin stellar-insights-backend
```

Failure mode to watch for: a startup that blocks indefinitely on the Vault HTTP
call instead of erroring out with a clear message.

## 3. Verify lease renewal over time

`get_database_credentials()` succeeding at startup says nothing about hour six.
Against a dev Vault with a short TTL, confirm renewal happens *before* expiry:

```bash
vault server -dev &
export VAULT_ADDR=http://127.0.0.1:8200
export VAULT_TOKEN=<dev-root-token>

# Short TTL so renewal is observable in minutes, not hours
vault write database/roles/stellar-insights default_ttl=2m max_ttl=10m ...

cargo run --bin stellar-insights-backend
```

Then watch the logs for at least three renewal cycles (~6 minutes):

- a renewal log line appears *before* each 2m TTL elapses,
- the database connection keeps working across renewals,
- at `max_ttl` the client re-issues credentials rather than dying.

```bash
vault lease list sys/leases/lookup/database/creds/stellar-insights
```

## 4. Revocation

On shutdown, leases should be revoked (`revoke_lease`) rather than left to
expire. Confirm the lease count above drops after a clean shutdown.

## Checklist

- [ ] `vault-deploy.yml` and `vault-deploy-approle.yml` green on `main`
- [ ] Dev startup with Vault unset succeeds via `DATABASE_URL` fallback
- [ ] Vault unreachable produces a clear error, not a hang or panic
- [ ] Lease renewal observed across ≥3 cycles before TTL expiry
- [ ] Leases revoked on clean shutdown
