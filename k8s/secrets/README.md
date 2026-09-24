# Secrets Management in Kubernetes

This directory provides production Kubernetes secret management configurations for PayRaider, supporting both **HashiCorp Vault** and **AWS Secrets Manager** via the **External Secrets Operator (ESO)**, as well as native Kubernetes Secrets with strict Role-Based Access Control (RBAC).

## Architecture

```
┌──────────────────────────────────────────────┐
│  Central Secret Store (Vault / AWS Secrets)   │
│  - KV v2 Engine / KMS encryption at rest      │
│  - Audit logging & 90-day rotation policy     │
└──────────────────────┬───────────────────────┘
                       │
                       │ sync via External Secrets Operator
                       ▼
┌──────────────────────────────────────────────┐
│  Kubernetes Secret (payraider-secrets)       │
│  - Namespace: payraider                      │
│  - Read restricted by RBAC Role & Binding    │
└──────────────────────┬───────────────────────┘
                       │
                       │ envFrom / volumeMount
                       ▼
┌──────────────────────────────────────────────┐
│  PayRaider Backend Pod (payraider-backend)   │
└──────────────────────────────────────────────┘
```

## Supported Integrations

### 1. HashiCorp Vault (`vault-secrets-store.yaml`)
Integrates with HashiCorp Vault KV v2 secrets engine using Kubernetes ServiceAccount authentication.
- Mounts Kubernetes service account tokens to authenticate against Vault.
- Automatically synchronizes secrets (`database-url`, `redis-url`, `jwt-secret`, `encryption-key`, etc.).
- Audit logs are recorded for every secret read operation.

### 2. AWS Secrets Manager (`aws-secrets-manager.yaml`)
Integrates with AWS Secrets Manager using IAM Roles for Service Accounts (IRSA) on Amazon EKS.
- Encrypted at rest using AWS KMS Customer Managed Keys (CMKs).
- Synchronizes values into `payraider-secrets` on configurable refresh intervals (default: `1h`).

### 3. Native Kubernetes Secrets & RBAC (`kubernetes-secrets.yaml`)
Provides a secured template with strict RBAC:
- `payraider-secret-reader` Role restricts secret `get` operations solely to `payraider-secrets`.
- `payraider-secret-reader-binding` binds this permission only to the `payraider-backend` ServiceAccount.

## Secret Rotation Policy (90-Day Rotation)

All production credentials MUST adhere to a **90-day maximum lifetime rotation policy**:
1. **JWT Secret (`jwt-secret`)**: Rotated every 90 days. During rotation, keep previous secret in secondary verification list for graceful session rollover.
2. **Encryption Key (`encryption-key`)**: Rotated every 90 days using envelope key re-wrapping.
3. **API Keys / Tokens**: Rotated via `scripts/rotate-vault-secrets.sh` or Vault CLI.

### Automated Rotation with Vault
Run the rotation script:
```bash
./scripts/rotate-vault-secrets.sh
```

## Audit Logging

HashiCorp Vault records immutable audit trails of all secret access:
```bash
# Enable file or syslog audit device
vault audit enable file file_path=/var/log/vault/audit.log

# Verify active audit backends
vault audit list

# Inspect secret access events
tail -f /var/log/vault/audit.log | jq '{time: .time, path: .request.path, client: .auth.display_name}'
```

## Verification

```bash
# 1. Verify secrets synchronized in cluster
kubectl get externalsecrets -n payraider
kubectl get secret payraider-secrets -n payraider -o yaml

# 2. Verify audit logs in Vault
vault audit list

# 3. Verify backend health
curl http://localhost:8080/health
```
