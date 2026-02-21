# ============================================================================
# ElastiCache Subnet Group
# ============================================================================

resource "aws_elasticache_subnet_group" "cache" {
  name       = var.cache_subnet_group_name
  subnet_ids = var.cache_subnet_ids

  tags = {
    Name = "stellar-insights-cache-subnet-${var.environment}"
  }
}

# ============================================================================
# ElastiCache Parameter Group
# ============================================================================

resource "aws_elasticache_parameter_group" "redis" {
  name   = "stellar-insights-redis-${var.environment}"
  family = var.parameter_group_family

  # Memory management
  parameter {
    name  = "maxmemory-policy"
    value = "allkeys-lru"  # Evict LRU keys when memory limit reached
  }

  # Connection management
  parameter {
    name  = "timeout"
    value = "300"  # Close idle connections after 5 minutes
  }

  # Persistence (disabled for performance)
  # Redis is used for caching only, not persistent storage
  parameter {
    name  = "save"
    value = ""  # Disable RDB snapshots for better performance
  }

  tags = {
    Name = "stellar-insights-redis-${var.environment}"
  }
}

# ============================================================================
# ElastiCache Redis Cluster
# ============================================================================

resource "aws_elasticache_cluster" "redis" {
  cluster_id           = var.cluster_id
  engine               = "redis"
  node_type            = var.node_type
  num_cache_nodes      = var.num_cache_nodes
  parameter_group_name = aws_elasticache_parameter_group.redis.name
  engine_version       = var.engine_version
  port                 = 6379

  # Networking
  subnet_group_name          = aws_elasticache_subnet_group.cache.name
  security_group_ids         = var.security_group_ids
  automatic_failover_enabled = var.automatic_failover_enabled

  # Maintenance and updates
  auto_minor_version_upgrade = var.auto_minor_version_upgrade
  maintenance_window         = var.environment == "dev" ? "sun:04:00-sun:05:00" : "sun:04:00-sun:05:00"

  # Snapshots
  snapshot_retention_limit = var.snapshot_retention_limit
  snapshot_window          = var.snapshot_window > 0 ? var.snapshot_window : null

  # Encryption
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  auth_token_enabled         = true
  auth_token                 = random_password.redis_auth_token.result

  # Notifications
  notification_topic_arn = try(aws_sns_topic.cache_notifications[0].arn, null)

  # Logging
  log_delivery_configuration {
    destination      = aws_cloudwatch_log_group.redis_slow_log.name
    destination_type = "cloudwatch-logs"
    log_format       = "json"
    log_type         = "slow-log"
    enabled          = var.environment != "dev"
  }

  tags = {
    Name = "stellar-insights-redis-${var.environment}"
  }

  depends_on = [
    aws_elasticache_parameter_group.redis,
    aws_elasticache_subnet_group.cache
  ]
}

# ============================================================================
# Auth Token for Redis
# ============================================================================

resource "random_password" "redis_auth_token" {
  length      = 32
  special     = true
  min_upper   = 1
  min_lower   = 1
  min_numeric = 1
}

# ============================================================================
# SNS Topic for ElastiCache Notifications (production only)
# ============================================================================

resource "aws_sns_topic" "cache_notifications" {
  count = var.environment == "production" ? 1 : 0
  name  = "stellar-insights-cache-notifications-${var.environment}"

  tags = {
    Name = "stellar-insights-cache-notifications-${var.environment}"
  }
}

# ============================================================================
# CloudWatch Log Groups
# ============================================================================

resource "aws_cloudwatch_log_group" "redis_slow_log" {
  name              = "/aws/elasticache/redis/${var.environment}/slow-log"
  retention_in_days = var.environment == "production" ? 14 : (var.environment == "staging" ? 7 : 3)

  tags = {
    Name = "stellar-insights-redis-logs-${var.environment}"
  }
}

# ============================================================================
# CloudWatch Alarms
# ============================================================================

resource "aws_cloudwatch_metric_alarm" "redis_cpu" {
  alarm_name          = "stellar-insights-redis-cpu-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "EngineCPUUtilization"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = var.environment == "production" ? "70" : "80"
  alarm_description   = "Alert when Redis CPU exceeds threshold"
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.redis.cluster_id
  }
}

resource "aws_cloudwatch_metric_alarm" "redis_memory" {
  alarm_name          = "stellar-insights-redis-memory-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "DatabaseMemoryUsagePercentage"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = "80"
  alarm_description   = "Alert when Redis memory exceeds 80%"
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.redis.cluster_id
  }
}

resource "aws_cloudwatch_metric_alarm" "redis_evictions" {
  alarm_name          = "stellar-insights-redis-evictions-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name         = "Evictions"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Sum"
  threshold           = "100"
  alarm_description   = "Alert when Redis evictions exceed threshold (memory pressure)"
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.redis.cluster_id
  }
}

resource "aws_cloudwatch_metric_alarm" "redis_swap_usage" {
  alarm_name          = "stellar-insights-redis-swap-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "1"
  metric_name         = "SwapUsage"
  namespace           = "AWS/ElastiCache"
  period              = "300"
  statistic           = "Average"
  threshold           = "52428800"  # 50MB
  alarm_description   = "Alert on Redis swap usage (critical!)"
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = aws_elasticache_cluster.redis.cluster_id
  }
}
