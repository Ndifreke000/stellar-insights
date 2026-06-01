terraform {
  required_version = ">= 1.5"

  backend "s3" {
    bucket         = "stellar-insights-terraform-state-ACCOUNT_ID"
    key            = "dev/terraform.tfstate"
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

# Get account ID for ARN construction
data "aws_caller_identity" "current" {}

# Get ECR repository URLs from global stack
data "aws_ecr_repository" "backend" {
  name = "stellar-insights-backend"
}

# ============================================================================
# NETWORKING (2 AZs for dev, cost-optimized)
# ============================================================================

module "networking" {
  source = "../../modules/networking"

  vpc_cidr                = var.vpc_cidr
  environment             = var.environment
  enable_nat_per_az       = false  # Single NAT for dev (cost: ~$30/month)
  enable_vpc_flow_logs    = false  # Disabled for dev (cost savings)
  azs                     = 2      # 2 AZs for dev
}

# ============================================================================
# CACHING (Redis for dev - single node, minimal cost)
# ============================================================================

module "caching" {
  source = "../../modules/caching"

  cache_subnet_group_name = "stellar-insights-cache-${var.environment}"
  cache_subnet_ids        = module.networking.private_db_subnet_ids
  security_group_ids      = [module.networking.security_group_redis_id]

  cluster_id               = "stellar-insights-${var.environment}"
  node_type               = "cache.t3.micro"
  num_cache_nodes         = 1
  engine_version          = "7.0"
  automatic_failover_enabled = false
  snapshot_retention_limit = 1

  environment = var.environment
}

# ============================================================================
# LOAD BALANCING (ALB with HTTPS redirect)
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

  # ACM Certificate (create manually first in AWS Console)
  # For dev testing, can use self-signed.
  # Path: AWS Console → ACM → Request certificate → import custom
  certificate_arn = "arn:aws:acm:${var.aws_region}:${data.aws_caller_identity.current.account_id}:certificate/REPLACE_WITH_CERT_ID"
  domain_name     = "dev-api.stellar-insights.local"

  # Logging disabled for dev cost savings
  enable_logs = false

  environment = var.environment
}

# ============================================================================
# COMPUTE - ECS CLUSTER
# ============================================================================

module "compute" {
  source = "../../modules/compute/ecs"

  cluster_name    = "stellar-insights-${var.environment}"
  container_image = "${data.aws_ecr_repository.backend.repository_url}:latest"
  container_port  = 8080
  container_cpu   = 256
  container_memory = 512

  # Fargate configuration (serverless - cost optimized for dev)
  launch_type     = "FARGATE"
  enable_fargate  = true

  desired_count = 1
  min_size      = 1
  max_size      = 2

  subnets         = module.networking.private_app_subnet_ids
  security_groups = [module.networking.security_group_backend_id]
  target_group_arn = module.load_balancing.target_group_arn

  # Configuration (dev uses SQLite, not RDS)
  vault_addr = var.vault_addr
  db_url     = "sqlite:///./stellar_insights.db"
  redis_url  = module.caching.redis_connection_string

  environment         = var.environment
  log_retention_days = 7
  enable_auto_scaling = false

  depends_on = [module.load_balancing]
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
# MONITORING (Minimal for dev)
# ============================================================================

module "monitoring" {
  source = "../../modules/monitoring"

  cluster_name = module.compute.cluster_name

  log_group_names = {
    ecs = module.compute.log_group_name
  }

  alarm_email      = var.alarm_email
  enable_dashboard = false
  enable_alarms    = false
  enable_cost_alerts = false  # Disabled for dev to save cost

  environment = var.environment
}

# ============================================================================
# OUTPUTS
# ============================================================================

output "alb_dns_name" {
  description = "ALB DNS name for accessing the API"
  value       = module.load_balancing.alb_dns_name
}

output "alb_zone_id" {
  description = "Zone ID for Route53 configuration"
  value       = module.load_balancing.alb_zone_id
}

output "ecs_cluster_name" {
  description = "ECS cluster name"
  value       = module.compute.cluster_name
}

output "ecs_service_name" {
  description = "ECS service name"
  value       = module.compute.service_name
}

output "redis_endpoint" {
  description = "Redis cluster endpoint"
  value       = module.caching.primary_endpoint
}

output "redis_auth_token" {
  description = "Redis AUTH token (store in Vault!)"
  value       = module.caching.auth_token
  sensitive   = true
}

output "vault_secret_paths" {
  description = "Secret paths in Vault KV v2"
  value       = module.vault.vault_secret_paths
}

output "cost_estimate" {
  description = "Estimated monthly cost (Fargate - cost optimized)"
  value = {
    description = "Full dev environment including NAT, ALB, ECS Fargate, Redis, CloudWatch"
    alb              = "$20/month"
    nat_gateway      = "$30/month"
    fargate_vcpu_hours = "$3/month"    # ~70 vCPU-hours @ $0.04/vCPU-hour
    fargate_memory_hours = "$2/month"  # ~200 GB-hours @ $0.008/GB-hour
    redis_cache      = "$5/month"
    data_transfer   = "$5/month"
    cloudwatch      = "<$1/month"
    total_monthly   = "~$65/month"
    savings_vs_ec2  = "~$3/month (negligible for dev)"
  }
}

# Next steps post-deployment:
# 1. ACM Certificate: Create or import SSL cert for domain (replace REPLACE_WITH_CERT_ID above)
# 2. Route53 (optional): Point your domain to ALB DNS name
# 3. Vault: Run setup-vault-complete.sh after creating HCP account
# 4. GitHub Actions: Configure VAULT_ADDR and VAULT_TOKEN in repo secrets
# 5. ECS Deployment: Push backend image to ECR, trigger deployment
# 6. Health Check: curl https://dev-api.stellar-insights.local/health
