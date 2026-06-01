output "primary_endpoint_address" {
  description = "Primary endpoint address (hostname)"
  value       = aws_elasticache_cluster.redis.cache_nodes[0].address
  sensitive   = false
}

output "primary_endpoint" {
  description = "Primary endpoint (hostname:port)"
  value       = "${aws_elasticache_cluster.redis.cache_nodes[0].address}:${aws_elasticache_cluster.redis.port}"
  sensitive   = false
}

output "port" {
  description = "Redis port"
  value       = aws_elasticache_cluster.redis.port
  sensitive   = false
}

output "cluster_id" {
  description = "ElastiCache cluster identifier"
  value       = aws_elasticache_cluster.redis.cluster_id
  sensitive   = false
}

output "engine_version" {
  description = "Redis engine version"
  value       = aws_elasticache_cluster.redis.engine_version
  sensitive   = false
}

output "auth_token" {
  description = "AUTH token for Redis connection (store in Vault!)"
  value       = random_password.redis_auth_token.result
  sensitive   = true
}

output "redis_connection_string" {
  description = "Redis connection string for Rust/Python (redis://token@host:port)"
  value       = "redis://:${random_password.redis_auth_token.result}@${aws_elasticache_cluster.redis.cache_nodes[0].address}:${aws_elasticache_cluster.redis.port}"
  sensitive   = true
}
