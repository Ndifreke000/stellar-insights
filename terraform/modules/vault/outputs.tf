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
