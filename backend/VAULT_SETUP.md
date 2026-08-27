# Vault Secrets Management Setup

## Overview

Stellar Insights uses HashiCorp Vault for secure secrets management. This document covers:
- Local development setup with Vault dev mode
- Production Vault configuration
- Secret paths and naming conventions
- Secret rotation procedures
- Troubleshooting

## Local Development Setup

### Using Docker Compose (Recommended)

Add Vault to your `docker-compose.yml`:

```yaml
vault:
  image: vault:latest
  ports:
    - "8200:8200"
  environment:
    VAULT_DEV_ROOT_TOKEN_ID: dev-token-12345
    VAULT_DEV_LISTEN_ADDRESS: 0.0.0.0:8200
  cap_add:
    - IPC_LOCK
  volumes:
    - ./vault-init.sh:/vault/init.sh
  command: server -dev -dev-root-token-id=dev-token-12345
```

### Manual Setup

Run Vault in dev mode:

```bash
vault server -dev -dev-root-token-id=dev-token-12345
```

This starts Vault on `http://127.0.0.1:8200` with the root token `dev-token-12345`.

### Configure Secrets

1. Export Vault credentials:

```bash
export VAULT_ADDR="http://127.0.0.1:8200"
export VAULT_TOKEN="dev-token-12345"
```

2. Create the KV v2 secrets engine (if not already created):

```bash
vault secrets enable -version=2 kv
```

3. Store application secrets:

```bash
vault kv put kv/app/secrets \
  jwt_secret="your-jwt-secret-at-least-32-chars" \
  encryption_key="your-encryption-key-64-hex-chars" \
  database_password="dev-password-optional"
```

4. Configure the backend to use Vault:

```bash
export VAULT_ADDR="http://127.0.0.1:8200"
export VAULT_TOKEN="dev-token-12345"
export VAULT_NAMESPACE=""
export DB_ROLE="stellar-app"
```

5. Run the backend:

```bash
cargo run
```

The application will initialize the `SecretsService` which will:
1. Attempt to connect to Vault using `VAULT_ADDR` and `VAULT_TOKEN`
2. Fetch secrets from `kv/app/secrets`
3. Fall back to environment variables (`JWT_SECRET`, `ENCRYPTION_KEY`) if Vault is unavailable

## Production Setup

### Prerequisites

- HashiCorp Vault cluster (v1.12 or later recommended)
- AppRole or Kubernetes auth method configured
- KV v2 secrets engine enabled at path `kv/`

### 1. Enable AppRole Authentication

```bash
vault auth enable approle

# Create a role for stellar-insights
vault write auth/approle/role/stellar-insights \
  token_ttl=1h \
  token_max_ttl=4h \
  secret_id_ttl=24h \
  bind_secret_id=true \
  secret_id_num_uses=0
```

### 2. Create Policies

Create a policy file `stellar-insights-policy.hcl`:

```hcl
path "kv/data/app/secrets" {
  capabilities = ["read"]
}

path "kv/metadata/app/secrets" {
  capabilities = ["read"]
}

path "database/creds/+/stellar-app" {
  capabilities = ["read"]
}

path "auth/token/renew-self" {
  capabilities = ["update"]
}

path "sys/leases/renew" {
  capabilities = ["update"]
}
```

Apply the policy:

```bash
vault policy write stellar-insights stellar-insights-policy.hcl
vault write auth/approle/role/stellar-insights policies=stellar-insights
```

### 3. Generate AppRole Credentials

```bash
# Get role ID
vault read auth/approle/role/stellar-insights/role-id

# Generate secret ID
vault write -f auth/approle/role/stellar-insights/secret-id
```

Save both values securely (use your infrastructure's secret management for CI/CD, Kubernetes secrets, etc.)

### 4. Create Secrets

```bash
vault kv put kv/app/secrets \
  jwt_secret="production-jwt-secret" \
  encryption_key="production-encryption-key" \
  database_password="production-db-password"
```

### 5. Configure Backend

Set environment variables on deployment:

```bash
# AppRole authentication
export VAULT_ADDR="https://vault.production.internal:8200"
export VAULT_ROLE_ID="your-role-id"
export VAULT_SECRET_ID="your-secret-id"
export VAULT_NAMESPACE="stellar-insights" # if using namespaces

# Database
export DB_ROLE="stellar-app"
```

**Note:** The current implementation uses token-based auth. For AppRole support, extend `VaultConfig::from_env()` to handle `VAULT_ROLE_ID` and `VAULT_SECRET_ID`, then authenticate using the AppRole endpoint before making secret requests.

## Secret Paths and Naming Convention

All application secrets are stored under `kv/app/secrets`:

| Secret | Vault Path | Environment Fallback |
|--------|-----------|--------|
| JWT Secret | `kv/app/secrets` → `jwt_secret` | `JWT_SECRET` |
| Encryption Key | `kv/app/secrets` → `encryption_key` | `ENCRYPTION_KEY` |
| Database Password | `kv/app/secrets` → `database_password` | `DATABASE_PASSWORD` |

### Example: Fetching Secrets

```rust
use stellar_insights_backend::vault::{SecretsService, VaultConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with Vault if available, else environment variables
    let secrets_service = SecretsService::new().await?;
    let secrets = secrets_service.get_secrets().await?;

    println!("JWT Secret: {}", secrets.jwt_secret);
    println!("Encryption Key: {}", secrets.encryption_key);
    // Database password is optional
    if let Some(pwd) = secrets.database_password {
        println!("Database Password: {}", pwd);
    }

    Ok(())
}
```

## Secret Rotation

### Manual Rotation

1. Generate new secret values
2. Update in Vault:

```bash
vault kv patch kv/app/secrets \
  jwt_secret="new-jwt-secret"
```

3. Restart the application (it re-fetches secrets on startup)

### Automated Rotation with Vault

For production, configure Vault's secret rotation policies:

```bash
# Enable PKI or other engines for automated rotation
vault secrets enable pki
vault secrets tune -max-lease-ttl=87600h pki

# Configure lease duration and rotation windows
vault kv metadata patch kv/app/secrets \
  -max-versions=5 \
  -delete-version-after=2592000s
```

### Monitoring Secret Age

Check when secrets were last updated:

```bash
vault kv metadata get kv/app/secrets
```

## Testing

Run the secrets service tests:

```bash
cd backend
cargo test vault::secrets_service
```

This verifies:
- Secrets can be fetched from environment variables (when Vault is unavailable)
- Secret structure is correct
- Error handling works as expected

## Troubleshooting

### Vault Unavailable

If Vault is unreachable during development:
1. The `SecretsService` will fall back to environment variables
2. Ensure `JWT_SECRET` and `ENCRYPTION_KEY` are set in `.env`

```bash
# .env file
JWT_SECRET="dev-secret-at-least-32-chars-long"
ENCRYPTION_KEY="dev-key-64-hex-chars-long"
VAULT_ADDR=""  # Empty or omitted to skip Vault
```

### "Secret not found" Error

1. Verify secrets are stored in Vault:

```bash
vault kv get kv/app/secrets
```

2. Check secret path matches configured path in `secrets_service.rs`
3. Verify Vault token has read permissions

### Token Expiration

If the Vault token expires in production:
1. The application should be restarted to re-authenticate
2. For long-lived apps, implement lease renewal (future enhancement)

## Security Considerations

1. **Token Management**: Never commit Vault tokens to version control. Use infrastructure secret management (Kubernetes Secrets, HashiCorp Consul, AWS Secrets Manager, etc.)
2. **Audit Logging**: Enable Vault audit logging to track all secret accesses
3. **Least Privilege**: Policies should grant only necessary permissions (read secrets, not write/delete)
4. **TLS**: Always use TLS for Vault communication in production (https://)
5. **Namespace Isolation**: Use Vault namespaces to isolate secrets per environment

## References

- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [Vault AppRole Authentication](https://www.vaultproject.io/docs/auth/approle)
- [Vault KV Secrets Engine](https://www.vaultproject.io/docs/secrets/kv)
- [Vault Production Hardening](https://www.vaultproject.io/docs/internals/security)
