terraform {
  required_version = ">= 1.5"

  backend "s3" {
    bucket         = "stellar-insights-terraform-state-ACCOUNT_ID"
    key            = "staging/terraform.tfstate"
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
# NETWORKING (2 AZs, Multi-AZ ready)
# ============================================================================

module "networking" {
  source = "../../modules/networking"

  vpc_cidr                = var.vpc_cidr
  environment             = var.environment
  enable_nat_per_az       = false  # Single NAT for cost (can enable for HA)
  enable_vpc_flow_logs    = true   # Enable for staging compliance
  azs                     = 2      # 2 AZs for staging
}

# ============================================================================
# DATABASE (RDS PostgreSQL - staging only)
# ============================================================================

module "database" {
  source = "../../modules/database"

  db_subnet_group_name = "stellar-insights-db-${var.environment}"
  vpc_security_group_ids = [module.networking.security_group_database_id]
  db_subnet_ids        = module.networking.private_db_subnet_ids

  identifier         = "stellar-insights-${var.environment}"
  instance_class     = "db.t3.small"
  allocated_storage  = 100
  storage_type       = "gp3"
  engine_version     = "14.8"

  multi_az                 = false  # Single-AZ for staging cost efficiency
  backup_retention_period  = 7
  enable_cloudwatch_logs_exports = ["postgresql"]
  enable_enhanced_monitoring = false

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# CACHING (Redis - single node for staging)
# ============================================================================

module "caching" {
  source = "../../modules/caching"

  cache_subnet_group_name = "stellar-insights-cache-${var.environment}"
  cache_subnet_ids        = module.networking.private_db_subnet_ids
  security_group_ids      = [module.networking.security_group_redis_id]

  cluster_id               = "stellar-insights-${var.environment}"
  node_type               = "cache.t3.small"
  num_cache_nodes         = 1
  engine_version          = "7.0"
  automatic_failover_enabled = false
  snapshot_retention_limit = 7

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# LOAD BALANCING (ALB with HTTPS)
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

  # ACM certificate (create manually first)
  certificate_arn = "arn:aws:acm:${var.aws_region}:${data.aws_caller_identity.current.account_id}:certificate/REPLACE_WITH_CERT_ID"
  domain_name     = "staging-api.stellar-insights.com"

  # Logging disabled for cost
  enable_logs = false

  # Blue-green deployment: creates green target group + test listener
  enable_blue_green = true

  environment = var.environment

  depends_on = [module.networking]
}

# ============================================================================
# COMPUTE - ECS CLUSTER
# ============================================================================

module "compute" {
  source = "../../modules/compute/ecs"

  cluster_name    = "stellar-insights-${var.environment}"
  container_image = "${data.aws_ecr_repository.backend.repository_url}:latest"
  container_port  = 8080
  container_cpu   = 512
  container_memory = 1024

  desired_count = 2
  min_size      = 2
  max_size      = 4
  instance_type = "t3.small"

  subnets         = module.networking.private_app_subnet_ids
  security_groups = [module.networking.security_group_backend_id]
  target_group_arn = module.load_balancing.target_group_arn

  # Blue-green deployment: switches to CODE_DEPLOY controller
  enable_blue_green = true

  # Configuration
  vault_addr = var.vault_addr
  db_url     = "postgresql://postgres@${module.database.rds_address}:5432/stellar_insights"
  redis_url  = module.caching.redis_connection_string

  environment         = var.environment
  log_retention_days = 14
  enable_auto_scaling = true
  cpu_target_percentage = 70

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
  deployment_config_name        = "CodeDeployDefault.ECSAllAtOnce"
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
# MONITORING (Standard for staging)
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
  description = "RDS PostgreSQL endpoint"
  value       = module.database.rds_endpoint
  sensitive   = false
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = module.caching.primary_endpoint
}

output "vault_secret_paths" {
  description = "Vault secret paths"
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
  description = "Estimated monthly cost for staging"
  value = {
    alb              = "$20/month"
    nat_gateway      = "$30/month"
    ecs_t3_small     = "$60/month"
    rds_t3_small     = "$60/month"
    redis_cache      = "$20/month"
    data_transfer    = "$10/month"
    cloudwatch_logs  = "$5/month"
    total_monthly    = "~$205/month"
  }
}

