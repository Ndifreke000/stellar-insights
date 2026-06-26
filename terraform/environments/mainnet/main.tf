terraform {
  required_version = ">= 1.5"

  backend "s3" {
    bucket         = "stellar-insights-terraform-state-ACCOUNT_ID"
    key            = "mainnet/terraform.tfstate"
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
# NETWORKING (3 AZs, Full HA - production grade)
# ============================================================================

module "networking" {
  source = "../../modules/networking"

  vpc_cidr                = var.vpc_cidr
  environment             = var.environment
  enable_nat_per_az       = true   # Multi-AZ NAT for mainnet HA
  enable_vpc_flow_logs    = true   # Full security monitoring
  azs                     = 3      # 3 AZs for geographic redundancy
}

# ============================================================================
# DATABASE (RDS PostgreSQL - Multi-AZ for mainnet)
# ============================================================================

module "database" {
  source = "../../modules/database"

  db_subnet_group_name = "stellar-insights-db-${var.environment}"
  vpc_security_group_ids = [module.networking.security_group_database_id]
  db_subnet_ids        = module.networking.private_db_subnet_ids

  identifier         = "stellar-insights-${var.environment}"
  instance_class     = "db.t3.medium"  # Mainnet minimum for transaction volume
  allocated_storage  = 500
  storage_type       = "gp3"
  engine_version     = "14.8"

  multi_az                 = true   # Automatic failover across AZs
  backup_retention_period  = 30     # 30-day retention for mainnet
  enable_cloudwatch_logs_exports = ["postgresql"]
  enable_enhanced_monitoring = true
  monitoring_interval      = 60

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# CACHING (Redis - Multi-AZ cluster)
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
  domain_name     = "api.mainnet.stellar-insights.com"

  # Enable request logging to S3
  enable_logs = true  # Important for mainnet audit trails

  # WAF can be enabled separately
  enable_waf = false  # Set to true if WAF Web ACL exists

  # Blue-green deployment
  enable_blue_green = true

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# COMPUTE - ECS CLUSTER (HA with aggressive auto-scaling)
# ============================================================================

module "compute" {
  source = "../../modules/compute/ecs"

  cluster_name    = "stellar-insights-${var.environment}"
  container_image = "${data.aws_ecr_repository.backend.repository_url}:latest"
  container_port  = 8080
  container_cpu   = 512
  container_memory = 1024

  # Fargate configuration (serverless)
  launch_type     = "FARGATE"
  enable_fargate  = true

  desired_count = 4
  min_size      = 4  # Minimum 4 tasks for mainnet HA
  max_size      = 20 # Scale up to 20 tasks under load

  subnets         = module.networking.private_app_subnet_ids
  security_groups = [module.networking.security_group_backend_id]
  target_group_arn = module.load_balancing.target_group_arn

  # Blue-green deployment
  enable_blue_green = true

  # Configuration
  vault_addr = var.vault_addr
  db_url     = "postgresql://postgres@${module.database.rds_address}:5432/stellar_insights"
  redis_url  = module.caching.redis_connection_string

  environment         = var.environment
  log_retention_days  = 90  # Longer retention for mainnet compliance
  enable_auto_scaling = true
  cpu_target_percentage = 60  # More aggressive scaling for mainnet

  depends_on = [module.load_balancing, module.database, module.caching]
}

# ============================================================================
# CODEDEPLOY (Blue-Green Deployment)
# ============================================================================

module "codedeploy" {
  source = "../../modules/codedeploy"

  cluster_name                  = module.compute.cluster_name
  service_name                  = module.compute.service_name
  listener_arn                  = module.load_balancing.https_listener_arn
  test_listener_arn             = module.load_balancing.test_listener_arn
  target_group_blue_name        = module.load_balancing.target_group_name
  target_group_green_name       = module.load_balancing.target_group_green_name
  deployment_config_name        = "CodeDeployDefault.ECSCanary10Percent5Minutes"
  alb_arn_suffix                = module.load_balancing.alb_arn_suffix
  target_group_blue_arn_suffix  = module.load_balancing.target_group_arn_suffix
  target_group_green_arn_suffix = module.load_balancing.target_group_green_arn_suffix
  environment                   = var.environment

  depends_on = [module.compute]
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
# MONITORING (Production-grade monitoring and alerting)
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
  description = "RDS PostgreSQL endpoint (Multi-AZ with failover)"
  value       = module.database.rds_endpoint
  sensitive   = false
}

output "redis_endpoints" {
  description = "Redis primary and replica endpoints (Multi-AZ cluster)"
  value       = module.caching.redis_connection_string
  sensitive   = true
}

output "vault_secret_paths" {
  description = "Vault secret paths for mainnet secrets"
  value       = module.vault.vault_secret_paths
}

output "codedeploy_app_name" {
  description = "CodeDeploy application name"
  value       = module.codedeploy.app_name
}

output "codedeploy_deployment_group_name" {
  description = "CodeDeploy deployment group name"
  value       = module.codedeploy.deployment_group_name
}

output "cost_estimate" {
  description = "Estimated monthly cost for mainnet (production-grade HA)"
  value = {
    alb                    = "$20/month"
    nat_gateway_per_az     = "$90/month"    # 3 NATs (one per AZ)
    fargate_vcpu_hours     = "$60/month"    # ~800 vCPU-hours @ $0.04/vCPU-hour
    fargate_memory_hours   = "$50/month"    # ~2000 GB-hours @ $0.008/GB-hour
    fargate_requests       = "$20/month"
    rds_t3_medium_multiaz  = "$200/month"   # Multi-AZ premium
    redis_3_node_multiaz   = "$50/month"
    data_transfer          = "$30/month"
    cloudwatch_logs        = "$20/month"    # 90-day retention
    alb_logging            = "$5/month"
    waf_optional           = "$5/month"
    total_monthly          = "~$450-500/month"
    notes                  = "Multi-AZ HA across 3 regions; autoscaling to 20 tasks"
  }
}

# ============================================================================
# PRE-DEPLOYMENT CHECKLIST
# ============================================================================
#
# [ ] Account and permissions properly configured
# [ ] VPC and networking deployed and tested
# [ ] Database: RDS Multi-AZ created, backups tested, restore procedure documented
# [ ] Cache: Redis 3-node cluster healthy, failover tested
# [ ] ALB: Health checks passing, HTTPS listener functional, WAF configured
# [ ] ECS: Tasks deploying successfully, logs flowing, auto-scaling tested
# [ ] Vault: Mainnet secrets configured, GitHub Actions authenticated
# [ ] Monitoring: CloudWatch dashboard configured, SNS notifications tested
# [ ] DNS: Route53 records pointing to ALB (mainnet domain)
# [ ] Security: Security groups properly configured, NACLs reviewed, no 0.0.0.0/0 to backend
# [ ] Load testing: Verified 1000+ req/sec, auto-scaling functional
# [ ] Disaster recovery: Backup/restore procedures tested, Multi-AZ failover validated
# [ ] On-call playbooks created and reviewed
# [ ] Team trained on mainnet incident response
# [ ] RTO/RPO defined and tested
#
# Post-deployment: Monitor dashboard 24/7, validate logs, test failover, document runbooks
