# ============================================================================
# KMS Key for RDS Encryption
# ============================================================================

resource "aws_kms_key" "rds" {
  description             = "KMS key for RDS encryption (${var.environment})"
  deletion_window_in_days = var.environment == "dev" ? 7 : 30
  enable_key_rotation     = true

  tags = {
    Name = "stellar-insights-rds-${var.environment}"
  }
}

resource "aws_kms_alias" "rds" {
  name          = "alias/stellar-insights-rds-${var.environment}"
  target_key_id = aws_kms_key.rds.key_id
}

# ============================================================================
# RDS DB Subnet Group
# ============================================================================

resource "aws_db_subnet_group" "database" {
  name       = var.db_subnet_group_name
  subnet_ids = var.db_subnet_ids

  tags = {
    Name = "stellar-insights-db-subnet-${var.environment}"
  }
}

# ============================================================================
# RDS Parameter Group
# ============================================================================

resource "aws_db_parameter_group" "postgresql" {
  name   = "stellar-insights-postgresql-${var.environment}"
  family = "postgres14"

  # Performance tuning for PostgreSQL 14
  parameter {
    name  = "shared_buffers"
    value = "{DBInstanceClassMemory/4}"
  }

  # Enable slow query logging for optimization
  parameter {
    name  = "log_min_duration_statement"
    value = "5000"  # Log queries taking >5s
  }

  # Connection management
  parameter {
    name  = "max_connections"
    value = var.environment == "dev" ? "100" : (var.environment == "staging" ? "250" : "500")
  }

  # Query planner
  parameter {
    name  = "random_page_cost"
    value = "1.1"  # SSD optimization
  }

  parameter {
    name  = "effective_cache_size"
    value = "{DBInstanceClassMemory*3/4}"
  }

  # SSL enforcement
  parameter {
    name         = "rds.force_ssl"
    value        = "1"
    apply_method = "pending-reboot"
  }

  # Timezone (UTC recommended)
  parameter {
    name  = "timezone"
    value = "UTC"
  }

  # Maintenance windows
  tags = {
    Name = "stellar-insights-pg14-${var.environment}"
  }
}

# ============================================================================
# IAM Role for Enhanced Monitoring
# ============================================================================

resource "aws_iam_role" "rds_enhanced_monitoring" {
  count = var.enable_enhanced_monitoring ? 1 : 0

  name = "stellar-insights-rds-monitoring-${var.environment}"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "monitoring.rds.amazonaws.com"
        }
      }
    ]
  })

  tags = {
    Name = "stellar-insights-rds-monitoring-${var.environment}"
  }
}

resource "aws_iam_role_policy_attachment" "rds_enhanced_monitoring" {
  count      = var.enable_enhanced_monitoring ? 1 : 0
  role       = aws_iam_role.rds_enhanced_monitoring[0].name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonRDSEnhancedMonitoringRole"
}

# ============================================================================
# RDS PostgreSQL Instance
# ============================================================================

resource "aws_db_instance" "postgresql" {
  identifier         = var.identifier
  engine             = "postgres"
  engine_version     = var.engine_version
  instance_class     = var.instance_class
  allocated_storage  = var.allocated_storage
  storage_type       = var.storage_type
  storage_encrypted  = true
  kms_key_id         = aws_kms_key.rds.arn

  # Database configuration
  db_name  = var.db_name
  username = var.username
  password = var.password

  # Networking
  publicly_accessible   = false
  db_subnet_group_name  = aws_db_subnet_group.database.name
  vpc_security_group_ids = var.vpc_security_group_ids

  # Parameter group
  parameter_group_name = aws_db_parameter_group.postgresql.name

  # High availability
  multi_az = var.multi_az

  # Backups
  backup_retention_period = var.backup_retention_period
  backup_window           = var.backup_window
  skip_final_snapshot     = var.skip_final_snapshot
  final_snapshot_identifier = var.skip_final_snapshot ? null : "stellar-insights-${var.environment}-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}"

  # Maintenance
  auto_minor_version_upgrade  = var.auto_minor_version_upgrade
  maintenance_window          = var.environment == "dev" ? "sun:04:00-sun:05:00" : "sun:04:00-sun:05:00"
  enable_iam_database_authentication = true

  # Cloudwatch logs
  enabled_cloudwatch_logs_exports = var.enable_cloudwatch_logs_exports

  # Enhanced monitoring
  enable_performance_insights       = var.environment != "dev"
  performance_insights_retention_period = var.environment == "production" ? 31 : 7
  monitoring_interval              = var.enable_enhanced_monitoring ? var.monitoring_interval : 0
  monitoring_role_arn              = var.enable_enhanced_monitoring ? aws_iam_role.rds_enhanced_monitoring[0].arn : null

  # GP3 specific settings
  iops              = var.storage_type == "gp3" ? var.iops : null
  storage_throughput = var.storage_type == "gp3" ? var.storage_throughput : null

  # Deletion protection for production
  deletion_protection = var.environment == "production" ? true : false

  tags = {
    Name = "stellar-insights-${var.environment}"
  }

  depends_on = [
    aws_db_parameter_group.postgresql,
    aws_db_subnet_group.database
  ]

  lifecycle {
    # Ignore snapshot identifier changes from provider
    ignore_changes = [final_snapshot_identifier]
  }
}

# ============================================================================
# CloudWatch Alarms
# ============================================================================

resource "aws_cloudwatch_metric_alarm" "rds_cpu" {
  alarm_name          = "stellar-insights-rds-cpu-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = var.environment == "production" ? "70" : "80"
  alarm_description   = "Alert when RDS CPU exceeds threshold"
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgresql.identifier
  }
}

resource "aws_cloudwatch_metric_alarm" "rds_connections" {
  alarm_name          = "stellar-insights-rds-connections-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = var.environment == "production" ? "400" : (var.environment == "staging" ? "200" : "80")
  alarm_description   = "Alert when database connections exceed threshold"
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgresql.identifier
  }
}

resource "aws_cloudwatch_metric_alarm" "rds_storage" {
  alarm_name          = "stellar-insights-rds-storage-${var.environment}"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = "1"
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = "300"
  statistic           = "Average"
  threshold           = var.allocated_storage * 1073741824 * 0.1  # Alert at 10% free space
  alarm_description   = "Alert when RDS free storage drops below 10%"
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgresql.identifier
  }
}

resource "aws_cloudwatch_metric_alarm" "rds_replication_lag" {
  count               = var.multi_az ? 1 : 0
  alarm_name          = "stellar-insights-rds-replication-lag-${var.environment}"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = "2"
  metric_name         = "AuroraBinlogReplicaLag"
  namespace           = "AWS/RDS"
  period              = "60"
  statistic           = "Maximum"
  threshold           = "1000"  # 1000ms
  alarm_description   = "Alert when Multi-AZ replication lag exceeds 1 second"
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = aws_db_instance.postgresql.identifier
  }
}

# ============================================================================
# CloudWatch Log Groups
# ============================================================================

resource "aws_cloudwatch_log_group" "rds_postgresql" {
  count             = contains(var.enable_cloudwatch_logs_exports, "postgresql") ? 1 : 0
  name              = "/aws/rds/instance/${var.identifier}/postgresql"
  retention_in_days = var.environment == "production" ? 30 : (var.environment == "staging" ? 14 : 7)

  tags = {
    Name = "stellar-insights-rds-logs-${var.environment}"
  }
}
