# ============================================================================
# Vault Integration (HCP Vault Cloud)
# ============================================================================
# 
# NOTE: This module documents the Vault integration pattern.
# Actual Vault setup (OIDC, AppRole, policies, secret engines) 
# happens via HCP Vault Cloud console or CLI.
# See: VAULT_QUICK_START.md and VAULT_INTEGRATION_GUIDE.md

# ============================================================================
# IAM Role for ECS Tasks to Access Vault via OIDC
# ============================================================================

resource "aws_iam_role" "vault_oidc" {
  name = "stellar-insights-vault-oidc-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRoleWithWebIdentity"
        Effect = "Allow"
        Principal = {
          Federated = "arn:aws:iam::aws:repo/github:Ndifreke000/stellar-insights:*"
        }
        Condition = {
          StringEquals = {
            "token.actions.githubusercontent.com:iss" = "https://token.actions.githubusercontent.com"
          }
        }
      }
    ]
  })

  tags = {
    Name = "stellar-insights-vault-oidc-${var.environment}"
  }
}

# IAM policy: allow OIDC tokens to access Vault
resource "aws_iam_role_policy" "vault_oidc_policy" {
  name = "vault-oidc-policy"
  role = aws_iam_role.vault_oidc.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "sts:GetCallerIdentity"
        ]
        Resource = "*"
      }
    ]
  })
}

# ============================================================================
# Documentation: Vault Secret Paths
# ============================================================================
#
# Configuration in HCP Vault Cloud (via console or CLI):
#
# 1. Enable Auth Methods:
#    Path: auth/jwt (for GitHub OIDC)
#    Path: auth/approle (for CI/CD fallback)
#
# 2. Enable Secret Engines:
#    Path: secret/ (KV v2)
#    Path: database/
#
# 3. Create Secrets (KV v2):
#    secret/stellar/jwt-secret - JWT signing key
#    secret/stellar/oauth-clients - OAuth client credentials
#    secret/stellar/webhooks - Zapier webhook config
#
# 4. Database Secret Engine:
#    Enable at path: database/
#    Create connection to RDS PostgreSQL
#    Create role: stellar-insights-${environment}
#    Dynamic credentials: 1 hour TTL
#
# 5. Policies:
#    stellar-app-policy: read secrets, get DB creds
#    stellar-ci-policy: rotate secrets, auth setup

# ============================================================================
# Outputs
# ============================================================================

output "vault_oidc_role_arn" {
  description = "ARN of Vault OIDC role for GitHub Actions"
  value       = aws_iam_role.vault_oidc.arn
}

output "vault_secret_paths" {
  description = "Secret paths in Vault KV v2"
  value = {
    jwt_secret      = "secret/stellar/jwt-secret"
    oauth_clients   = "secret/stellar/oauth-clients"
    webhooks        = "secret/stellar/webhooks"
    db_role         = "database/creds/stellar-insights-${var.environment}"
  }
}
