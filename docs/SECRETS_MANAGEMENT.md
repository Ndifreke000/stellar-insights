# Secrets Management Strategy

## Overview

PayRaider implements a comprehensive secrets management solution using HashiCorp Vault for secure storage, rotation, and audit logging of sensitive credentials.

## Architecture

### Components

1. **HashiCorp Vault** - Central secrets store
2. **Vault Agent** - Automatic secret injection
3. **Audit Logging** - Track all secret access
4. **Secret Rotation** - Automatic 90-day rotation
5. **Kubernetes Secrets** - Production secret storage

### Deployment Topology

```
┌─────────────────────────────────────────┐
│         Application Pods                │
│  ┌──────────────────────────────────┐   │
│  │  Vault Agent (sidecar)           │   │
│  │  - Injects secrets               │   │
│  │  - Handles rotation              │   │
│  │  - Logs access                   │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
           │
           ▼
┌─────────────────────────────────────────┐
│      HashiCorp Vault                    │
│  ┌──────────────────────────────────┐   │
│  │  KV Secrets Engine               │   │
│  │  - Database credentials          │   │
│  │  - API keys                      │   │
│  │  - Encryption keys               │   │
│  └──────────────────────────────────┘   │
│  ┌──────────────────────────────────┐   │
│  │  Audit Log                       │   │
│  │  - All access tracked            │   │
│  │  - Immutable records             │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

## Setup

### 1. Install Vault

```bash
# macOS
brew install vault

# Linux
wget https://releases.hashicorp.com/vault/1.15.0/vault_1.15.0_linux_amd64.zip
unzip vault_1.15.0_linux_amd64.zip
sudo mv vault /usr/local/bin/
```

### 2. Start Vault Server

```bash
# Development mode (not for production)
vault server -dev

# Production mode (requires configuration)
vault server -config=/etc/vault/config.hcl
```

### 3. Initialize Vault

```bash
# Set Vault address
export VAULT_ADDR='http://127.0.0.1:8200'

# Initialize Vault (generates unseal keys)
vault operator init -key-shares=5 -key-threshold=3

# Unseal Vault (requires 3 of 5 keys)
vault operator unseal <key1>
vault operator unseal <key2>
vault operator unseal <key3>

# Authenticate
vault login <root-token>
```

## Secret Storage

### Enable KV Secrets Engine

```bash
vault secrets enable -path=secret kv-v2
```

### Store Secrets

```bash
# Database credentials
vault kv put secret/database/postgres \
  username=postgres \
  password=secure-password \
  host=db.example.com \
  port=5432

# API keys
vault kv put secret/api/stellar \
  rpc_url=https://rpc.stellar.org \
  api_key=your-api-key

# Encryption keys
vault kv put secret/encryption \
  master_key=your-master-key \
  jwt_secret=your-jwt-secret
```

### Retrieve Secrets

```bash
# Get all secrets
vault kv get secret/database/postgres

# Get specific field
vault kv get -field=password secret/database/postgres

# Get as JSON
vault kv get -format=json secret/database/postgres
```

## Secret Rotation

### Configure Rotation Policy

```bash
# Enable database secrets engine
vault secrets enable database

# Configure PostgreSQL connection
vault write database/config/postgresql \
  plugin_name=postgresql-database-plugin \
  allowed_roles="readonly,readwrite" \
  connection_url="postgresql://{{username}}:{{password}}@db.example.com:5432/postgres" \
  username="vault" \
  password="vault-password"

# Create rotation policy (90 days)
vault write database/roles/readwrite \
  db_name=postgresql \
  creation_statements="CREATE ROLE \"{{name}}\" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}';" \
  default_ttl="2160h" \
  max_ttl="2592h"
```

### Automatic Rotation

```bash
# Enable automatic rotation
vault write sys/leases/lookup/database/creds/readwrite \
  increment=2160h

# Check rotation status
vault read database/creds/readwrite
```

## Audit Logging

### Enable Audit Logging

```bash
# Enable file audit backend
vault audit enable file file_path=/var/log/vault/audit.log

# Enable syslog audit backend
vault audit enable syslog tag="vault" facility="LOCAL7"
```

### View Audit Logs

```bash
# List audit backends
vault audit list

# View audit logs
tail -f /var/log/vault/audit.log

# Query specific events
vault audit list -detailed
```

### Audit Log Format

```json
{
  "time": "2024-04-24T18:39:47.178Z",
  "type": "request",
  "auth": {
    "client_token": "s.xxxxxxxxxxxxxxxx",
    "accessor": "xxxxxxxxxxxxxxxx",
    "display_name": "kubernetes-auth",
    "policies": ["default", "app-policy"],
    "token_ttl": 3600,
    "token_type": "service"
  },
  "request": {
    "id": "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx",
    "operation": "read",
    "path": "secret/data/database/postgres",
    "data": {},
    "remote_address": "10.0.0.1"
  },
  "response": {
    "auth": null,
    "data": {
      "data": {
        "password": "***",
        "username": "***"
      }
    }
  },
  "error": ""
}
```

## Kubernetes Integration

### 1. Install Vault Agent Injector

```bash
helm repo add hashicorp https://helm.releases.hashicorp.com
helm install vault hashicorp/vault \
  --set injector.enabled=true \
  --set server.dataStorage.size=10Gi
```

### 2. Configure Kubernetes Auth

```bash
# Enable Kubernetes auth method
vault auth enable kubernetes

# Configure Kubernetes auth
vault write auth/kubernetes/config \
  kubernetes_host="https://$KUBERNETES_SERVICE_HOST:$KUBERNETES_SERVICE_PORT" \
  kubernetes_ca_cert=@/var/run/secrets/kubernetes.io/serviceaccount/ca.crt \
  token_reviewer_jwt=@/var/run/secrets/kubernetes.io/serviceaccount/token
```

### 3. Create Policy

```bash
# Create policy for app
vault policy write app-policy - <<EOF
path "secret/data/database/*" {
  capabilities = ["read", "list"]
}
path "secret/data/api/*" {
  capabilities = ["read", "list"]
}
path "secret/data/encryption/*" {
  capabilities = ["read"]
}
EOF
```

### 4. Create Kubernetes Role

```bash
vault write auth/kubernetes/role/app \
  bound_service_account_names=app \
  bound_service_account_namespaces=default \
  policies=app-policy \
  ttl=24h
```

### 5. Annotate Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: payraider-backend
spec:
  template:
    metadata:
      annotations:
        vault.hashicorp.com/agent-inject: "true"
        vault.hashicorp.com/role: "app"
        vault.hashicorp.com/agent-inject-secret-database: "secret/data/database/postgres"
        vault.hashicorp.com/agent-inject-template-database: |
          {{- with secret "secret/data/database/postgres" -}}
          export DB_USER="{{ .Data.data.username }}"
          export DB_PASSWORD="{{ .Data.data.password }}"
          export DB_HOST="{{ .Data.data.host }}"
          export DB_PORT="{{ .Data.data.port }}"
          {{- end }}
    spec:
      serviceAccountName: app
      containers:
      - name: backend
        image: payraider:latest
        env:
        - name: DATABASE_URL
          value: "postgresql://$(DB_USER):$(DB_PASSWORD)@$(DB_HOST):$(DB_PORT)/payraider"
```

## Environment Variables

### Development

```bash
# Use .env file (not in version control)
DATABASE_URL=postgresql://user:password@localhost:5432/payraider
STELLAR_RPC_URL=https://rpc.stellar.org
API_KEY=dev-api-key
```

### Production

```bash
# Vault-injected secrets
# No secrets in environment variables
# All secrets loaded from Vault Agent
```

## Best Practices

### 1. Never Commit Secrets

```bash
# Add to .gitignore
echo ".env" >> .gitignore
echo ".env.local" >> .gitignore
echo "secrets/" >> .gitignore
```

### 2. Use Short-Lived Tokens

```bash
# Set token TTL to 1 hour
vault token create -ttl=1h

# Renew token before expiration
vault token renew
```

### 3. Rotate Secrets Regularly

```bash
# Rotate database password every 90 days
vault write -f database/rotate-root/postgresql

# Rotate API keys
vault kv put secret/api/stellar api_key=new-key
```

### 4. Audit Access

```bash
# Review who accessed secrets
grep "secret/database" /var/log/vault/audit.log

# Alert on suspicious access
# Configure monitoring for failed auth attempts
```

### 5. Encrypt Secrets at Rest

```bash
# Enable encryption in Vault config
seal "awskms" {
  region     = "us-east-1"
  kms_key_id = "arn:aws:kms:us-east-1:123456789012:key/12345678-1234-1234-1234-123456789012"
}
```

## Troubleshooting

### Vault Sealed

```bash
# Check status
vault status

# Unseal with keys
vault operator unseal <key1>
vault operator unseal <key2>
vault operator unseal <key3>
```

### Authentication Failed

```bash
# Check token
vault token lookup

# Renew token
vault token renew

# Re-authenticate
vault login -method=kubernetes role=app
```

### Secret Not Found

```bash
# List available secrets
vault kv list secret/

# Check path
vault kv get secret/database/postgres

# Verify permissions
vault policy read app-policy
```

## References

- [HashiCorp Vault Documentation](https://www.vaultproject.io/docs)
- [Vault Kubernetes Auth](https://www.vaultproject.io/docs/auth/kubernetes)
- [Vault Agent Injector](https://www.vaultproject.io/docs/platform/k8s/injector)
- [Secret Rotation Best Practices](https://www.vaultproject.io/docs/secrets/databases)
