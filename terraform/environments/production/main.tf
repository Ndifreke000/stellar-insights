terraform {
  required_version = ">= 1.5"

  backend "s3" {
    bucket         = "stellar-insights-terraform-state-ACCOUNT_ID"
    key            = "production/terraform.tfstate"
    region         = "us-east-1"
    dynamodb_table = "terraform-locks"
    encrypt        = true
  }

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Environment = var.environment
      Project     = "stellar-insights"
      ManagedBy   = "terraform"
      CreatedAt   = timestamp()
    }
  }
}

# Get account ID and ECR repositories
data "aws_caller_identity" "current" {}
data "aws_ecr_repository" "backend" {
  name = "stellar-insights-backend"
}

# ============================================================================
# NETWORKING (3 AZs, Full HA)
# ============================================================================

module "networking" {
  source = "../../modules/networking"

  vpc_cidr                = var.vpc_cidr
  environment             = var.environment
  enable_nat_per_az       = true   # Per-AZ NAT for HA
  enable_vpc_flow_logs    = true   # Enable for security & compliance
  azs                     = 3      # 3 AZs for full HA
}

# ============================================================================
# DATABASE (RDS PostgreSQL - Multi-AZ)
# ============================================================================

module "database" {
  source = "../../modules/database"

  db_subnet_group_name = "stellar-insights-db-${var.environment}"
  vpc_security_group_ids = [module.networking.security_group_database_id]
  db_subnet_ids        = module.networking.private_db_subnet_ids

  identifier         = "stellar-insights-${var.environment}"
  instance_class     = "db.t3.small"
  allocated_storage  = 500
  storage_type       = "gp3"
  engine_version     = "14.8"

  multi_az                 = true   # Full failover
  backup_retention_period  = 30     # 30-day retention
  enable_cloudwatch_logs_exports = ["postgresql"]
  enable_enhanced_monitoring = true
  monitoring_interval      = 60

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# CACHING (Redis - Multi-AZ)
# ============================================================================

module "caching" {
  source = "../../modules/caching"

  cache_subnet_group_name = "stellar-insights-cache-${var.environment}"
  cache_subnet_ids        = module.networking.private_db_subnet_ids
  security_group_ids      = [module.networking.security_group_redis_id]

  cluster_id               = "stellar-insights-${var.environment}"
  node_type               = "cache.t3.small"
  num_cache_nodes         = 3  # Multi-AZ with replicas
  engine_version          = "7.0"
  automatic_failover_enabled = true
  snapshot_retention_limit = 14

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# LOAD BALANCING (ALB with HTTPS + WAF)
# ============================================================================

module "load_balancing" {
  source = "../../modules/load_balancing"

  name               = "stellar-insights-alb-${var.environment}"
  internal           = false
  load_balancer_type = "application"
  subnets            = module.networking.public_subnet_ids
  security_groups    = [module.networking.security_group_alb_id]

  target_group_name = "stellar-insights-targets-${var.environment}"
  target_port       = 8080

  # ACM certificate (wildcard or specific domain)
  certificate_arn = "arn:aws:acm:${var.aws_region}:${data.aws_caller_identity.current.account_id}:certificate/REPLACE_WITH_CERT_ID"
  domain_name     = "api.stellar-insights.com"

  # Enable request logging to S3
  enable_logs = false  # Set to true if S3 bucket exists for ALB logs

  # WAF can be enabled separately
  enable_waf = false   # Set to true if WAF Web ACL exists

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# COMPUTE - ECS CLUSTER (HA with auto-scaling)
# ============================================================================

module "compute" {
  source = "../../modules/compute/ecs"

  cluster_name    = "stellar-insights-${var.environment}"
  container_image = "${data.aws_ecr_repository.backend.repository_url}:latest"
  container_port  = 8080
  container_cpu   = 512
  container_memory = 1024

  desired_count = 3
  min_size      = 3  # Always at least 3 for HA
  max_size      = 10 # Scale up for traffic spikes
  instance_type = "t3.small"

  subnets         = module.networking.private_app_subnet_ids
  security_groups = [module.networking.security_group_backend_id]
  target_group_arn = module.load_balancing.target_group_arn

  # Configuration
  vault_addr = var.vault_addr
  db_url     = "postgresql://postgres@${module.database.rds_address}:5432/stellar_insights"
  redis_url  = module.caching.redis_connection_string

  environment         = var.environment
  log_retention_days = 30  # Full retention for production
  enable_auto_scaling = true
  cpu_target_percentage = 70

  depends_on = [module.load_balancing, module.database, module.caching]
}

# ============================================================================
# VAULT INTEGRATION
# ============================================================================

module "vault" {
  source = "../../modules/vault"

  vault_addr  = var.vault_addr
  environment = var.environment
}

# ============================================================================
# MONITORING (Full monitoring for production)
# ============================================================================

module "monitoring" {
  source = "../../modules/monitoring"

  cluster_name = module.compute.cluster_name

  log_group_names = {
    ecs = module.compute.log_group_name
  }

  alarm_email      = var.alarm_email
  enable_dashboard = true
  enable_alarms    = true

  environment = var.environment
}

# ============================================================================
# OUTPUTS
# ============================================================================

output "alb_dns_name" {
  description = "ALB DNS name for Route53"
  value       = module.load_balancing.alb_dns_name
}

output "database_endpoint" {
  description = "RDS PostgreSQL endpoint (Multi-AZ)"
  value       = module.database.rds_endpoint
  sensitive   = false
}

output "redis_endpoints" {
  description = "Redis primary and replica endpoints"
  value       = module.caching.redis_connection_string
  sensitive   = true
}

output "vault_secret_paths" {
  description = "Vault secret paths"
  value       = module.vault.vault_secret_paths
}

output "cost_estimate" {
  description = "Estimated monthly cost for production (HA)"
  value = {
    alb                = "$20/month"
    nat_gateways_3az   = "$90/month"  # $30 × 3
    ecs_t3_small_3x    = "$90/month"
    ecs_autoscaling    = "$30/month"  # Additional scaling capacity
    rds_t3_small_multiaz = "$150/month"
    redis_3_node_multiaz = "$40/month"
    data_transfer      = "$20/month"
    cloudwatch_logs    = "$10/month"
    waf_optional       = "$5/month"
    total_monthly      = "~$455/month"
  }
}

# ============================================================================
# PRE-DEPLOYMENT CHECKLIST
# ============================================================================
#
# [ ] VPC and networking deployed and tested
# [ ] Database: RDS created, backups tested, restore procedure documented
# [ ] Cache: Redis cluster healthy, failover tested
# [ ] ALB: Health checks passing, HTTPS listener functional
# [ ] ECS: Tasks deploying successfully, logs flowing
# [ ] Vault: Secrets configured, GitHub Actions authenticated
# [ ] Monitoring: Alarms configured, SNS notifications tested
# [ ] DNS: Route53 records pointing to ALB
# [ ] Security: Security groups properly configured, NACLs reviewed
# [ ] Load testing: Verified 100+ req/sec, auto-scaling functional
# [ ] Disaster recovery: Backup/restore procedures tested
# [ ] On-call playbooks created and reviewed
# [ ] Team trained on incident response
#
# Post-deployment: Monitor CloudWatch dashboard, validate logs, test failover

