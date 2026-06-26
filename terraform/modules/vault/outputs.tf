output "vault_oidc_role_arn" {
  description = "ARN of Vault OIDC role for GitHub Actions"
  value       = aws_iam_role.vault_oidc.arn
}

output "vault_secret_paths" {
  description = "Secret paths in Vault KV v2"
  value = {
    jwt_secret    = "secret/stellar/jwt-secret"
    oauth_clients = "secret/stellar/oauth-clients"
    webhooks      = "secret/stellar/webhooks"
    db_role       = "database/creds/stellar-insights-${var.environment}"
  }
}

output "vault_cluster_config" {
  description = "Vault HA cluster configuration"
  value = {
    ha_enabled      = var.ha_enabled
    cluster_size    = local.vault_node_count
    storage_backend = var.storage_backend
  }
}

output "vault_service_name" {
  description = "ECS service name for the Vault HA cluster (when subnet_ids provided)"
  value       = length(aws_ecs_service.vault) > 0 ? aws_ecs_service.vault[0].name : null
}
