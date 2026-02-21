# Vault Module

Manages HashiCorp Vault integration for Stellar Insights (assuming HCP Vault Cloud).

## Features

- IAM authentication method setup (OIDC)
- AppRole authentication method (alternative)
- Vault policies for application access
- Database secret engine configuration
- KV v2 secrets engine
- Audit logging

## Architecture

**Authentication Methods:**
1. **OIDC (Recommended)**: GitHub Actions → HCP Vault (no credential management)
2. **AppRole (Fallback)**: Long-lived credentials for CI/CD without GitHub

**Secret Paths (KV v2):**
- `secret/stellar/jwt-secret` - JWT signing key
- `secret/stellar/oauth-clients` - OAuth client credentials
- `secret/stellar/webhooks` - Zapier webhook signing keys

**Database Credentials:**
- PostgreSQL dynamic credentials via database secret engine
- TTL: 1 hour (auto-renewed by app)

## Usage

This module is primarily for reference and policy validation. Most setup happens via:

1. HCP Vault Console (UI):
   - Enable OIDC auth method
   - Create AppRole
   - Create policies
   - Enable KV v2 secrets engine
   - Enable database secret engine
   - Create database connection

2. Terraform (this module):
   - Reference existing Vault configuration
   - Create IAM roles for ECS tasks
   - Document secret paths

## Manual Setup Steps

See [VAULT_QUICK_START.md](../../VAULT_QUICK_START.md) and [VAULT_INTEGRATION_GUIDE.md](../../VAULT_INTEGRATION_GUIDE.md)

## Inputs

| Name | Description | Type |
|------|-------------|------|
| vault_addr | Vault server address | `string` |
| vault_token | Vault root token (for setup only) | `string` |
| environment | Environment name | `string` |

## Notes

- HCP Vault Cloud is external to Terraform
- This module documents the integration pattern
- Actual secret setup happens via Vault CLI or HCP UI
- See backend/src/vault/ for application implementation
- GitHub Actions workflows: vault-deploy.yml (OIDC), vault-deploy-approle.yml (AppRole)
