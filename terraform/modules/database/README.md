# Database Module

Manages AWS RDS PostgreSQL instances for Stellar Insights backend.

## Features

- PostgreSQL 14+ engine with automatic patching
- Automatic backups with configurable retention (7-30 days)
- CloudWatch monitoring and alarms
- Parameter groups for performance tuning
- Enhanced monitoring with IAM role
- Encrypted storage (KMS)
- Multi-AZ option for high availability
- Automatic failover in Multi-AZ deployments

## Usage

```hcl
module "database" {
  source = "../../modules/database"

  # Networking
  db_subnet_group_name            = aws_db_subnet_group.database.name
  vpc_security_group_ids          = [aws_security_group.database.id]
  
  # Instance configuration
  identifier          = "stellar-insights-${var.environment}"
  instance_class      = "db.t3.small"  # t3.micro for dev, t3.small for staging/prod
  allocated_storage   = 100           # GB
  storage_type        = "gp3"
  
  # Database
  engine              = "postgres"
  engine_version      = "14.8"
  db_name            = "stellar_insights"
  username            = "postgres"
  password            = random_password.db.result  # From Vault in production
  
  # HA and backup
  multi_az            = var.environment == "production"
  backup_retention    = var.environment == "dev" ? 7 : (var.environment == "staging" ? 14 : 30)
  skip_final_backup   = var.environment == "dev"
  
  # Monitoring
  enable_cloudwatch_logs_exports = ["postgresql"]
  enable_enhanced_monitoring      = var.environment != "dev"
  
  # Tagging
  environment         = var.environment
  project            = "stellar-insights"
}
```

## Inputs

| Name | Description | Type | Required |
|------|-------------|------|----------|
| db_subnet_group_name | DB subnet group name | `string` | Yes |
| vpc_security_group_ids | Security group IDs | `list(string)` | Yes |
| identifier | RDS instance identifier | `string` | Yes |
| instance_class | RDS instance class | `string` | Yes (e.g., `db.t3.micro`, `db.t3.small`) |
| allocated_storage | Allocated storage in GB | `number` | Yes |
| storage_type | Storage type (gp3, gp2, io1) | `string` | No (default: `gp3`) |
| engine_version | PostgreSQL version | `string` | No (default: `14.8`) |
| db_name | Initial database name | `string` | No (default: `stellar_insights`) |
| username | Master username | `string` | No (default: `postgres`) |
| password | Master password | `string` | Yes (use Vault in production) |
| multi_az | Enable Multi-AZ deployment | `bool` | No (default: `false`) |
| backup_retention | Backup retention in days | `number` | No (default: `7`) |
| skip_final_backup | Skip final backup on destroy | `bool` | No (default: `false`) |
| enable_cloudwatch_logs_exports | Export logs to CloudWatch | `list(string)` | No |
| enable_enhanced_monitoring | Enable enhanced monitoring | `bool` | No (default: `false`) |
| monitoring_interval | Enhanced monitoring interval in seconds | `number` | No (default: `60`) |
| environment | Environment name | `string` | Yes |
| project | Project name | `string` | No (default: `stellar-insights`) |

## Outputs

| Name | Description |
|------|-------------|
| rds_endpoint | RDS endpoint address (hostname) |
| rds_port | RDS port (default 5432) |
| rds_arn | RDS instance ARN |
| security_group_id | Database security group ID |
| resource_id | RDS resource ID (for IAM policies) |

## Cost Estimates

**Development:**
- Instance: db.t3.micro (~$10/month)
- Storage: 20GB GP3 (~$2/month)
- Backup: 7 days (~$1/month)
- No Multi-AZ, minimal monitoring
- Monthly: ~$13/month

**Staging:**
- Instance: db.t3.small (~$30/month)
- Storage: 100GB GP3 (~$10/month)
- Backup: 14 days (~$3/month)
- Single-AZ, standard monitoring
- Monthly: ~$43/month

**Production:**
- Instance: db.t3.small Multi-AZ (~$60/month)
- Storage: 500GB GP3 (~$50/month)
- Backup: 30 days (~$15/month)
- Multi-AZ failover, enhanced monitoring
- Monthly: ~$125/month

## Notes

- PostgreSQL 14 requires no special Stellar-specific configuration
- Heroku/AWS native PostgreSQL compatible
- Connection pooling handled by application (HikariCP in Java, sqlx in Rust)
- Automated upgrades enabled for minor versions only
- Database credentials managed via HashiCorp Vault (backend/src/vault/client.rs)
- See [VAULT_INTEGRATION_GUIDE.md](../../VAULT_INTEGRATION_GUIDE.md) for database secret engine configuration
