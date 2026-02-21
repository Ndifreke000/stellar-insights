# Caching Module

Manages AWS ElastiCache Redis clusters for Stellar Insights caching layer.

## Features

- Redis 7.x engine with automatic patching
- Automatic backup and failover
- CloudWatch monitoring and alarms  
- Parameter groups for performance tuning
- Multi-AZ replication (production only)
- Automatic failover with read replicas
- VPC endpoint for secure access
- Encryption at rest and in transit

## Usage

```hcl
module "caching" {
  source = "../../modules/caching"

  # Networking
  cache_subnet_group_name = aws_elasticache_subnet_group.cache.name
  security_group_ids      = [aws_security_group.redis.id]
  
  # Instance configuration
  cluster_id      = "stellar-insights-${var.environment}"
  node_type       = "cache.t3.small"  # cache.t3.micro for dev
  num_cache_nodes = var.environment == "production" ? 3 : 1
  engine_version  = "7.0"
  
  # Automatic failover
  automatic_failover_enabled = var.environment == "production"
  
  # Backup
  snapshot_retention_limit = var.environment == "dev" ? 1 : (var.environment == "staging" ? 7 : 14)
  snapshot_window          = "03:00-04:00"
  
  environment = var.environment
}
```

## Inputs

| Name | Description | Type | Required |
|------|-------------|------|----------|
| cache_subnet_group_name | ElastiCache subnet group name | `string` | Yes |
| security_group_ids | Security group IDs | `list(string)` | Yes |
| cluster_id | ElastiCache cluster identifier | `string` | Yes |
| node_type | Node instance type (cache.t3.*) | `string` | Yes |
| num_cache_nodes | Number of cache nodes | `number` | Yes |
| engine_version | Redis engine version | `string` | No (default: `7.0`) |
| automatic_failover_enabled | Enable automatic failover | `bool` | No (default: `false`) |
| snapshot_retention_limit | Backup retention in days | `number` | No (default: `0`) |
| snapshot_window | Backup window (UTC) | `string` | No |
| automatic_failover_enabled | Enable auto-failover for Multi-AZ | `bool` | No |
| environment | Environment name | `string` | Yes |

## Outputs

| Name | Description |
|------|-------------|
| primary_endpoint | Primary endpoint address:port |
| reader_endpoint | Reader endpoint address:port (if Multi-AZ) |
| configuration_endpoint | Configuration endpoint for cluster mode |
| port | Redis port (default 6379) |
| security_group_id | Security group ID |

## Cost Estimates

**Development:**
- Node: cache.t3.micro (~$5/month)
- 1 node, no replication
- Minimal monitoring
- Monthly: ~$5/month

**Staging:**
- Node: cache.t3.small (~$15/month)
- 1 node, daily snapshot (backups ~$1/month)
- Standard monitoring
- Monthly: ~$16/month

**Production:**
- Nodes: cache.t3.small × 3 (~$45/month)
- Multi-AZ with 2 replicas, automatic failover
- 14-day snapshot retention (~$3/month)
- Enhanced monitoring
- Monthly: ~$48/month

## Notes

- Redis data is volatile by design (session cache only)
- Persistence not recommended for performance
- Use for: session tokens, rate limits, temporary caches
- DO NOT use for persistent data (use RDS PostgreSQL)
- Evergreen updates enabled for automatic patching
- No read replicas in dev (cost optimization)
