# Terraform Development Environment Configuration
# 
# This environment uses minimal resources for cost efficiency
# Cost estimate: ~$50-70/month
# - VPC/networking: ~$10 (NAT gateway is the main cost)
# - RDS: skipped (uses local SQLite)
# - ElastiCache: skipped or micro instance
# - ECS: 1 t3.micro instance
# - ALB: ~$20/month
# - Data transfer: ~$10

terraform {
  required_version = ">= 1.5"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Configure remote state after terraform/global/ is applied
  backend "s3" {
    region         = "us-east-1"
    encrypt        = true
    dynamodb_table = "terraform-locks"

    # Set bucket name when applying: terraform init -backend-config="bucket=YOUR_BUCKET"
  }
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Environment = "dev"
      Project     = "stellar-insights"
      ManagedBy   = "Terraform"
      CostCenter  = "Development"
    }
  }
}

# Call networking module
module "networking" {
  source = "../../modules/networking"

  environment           = var.environment
  aws_region            = var.aws_region
  vpc_cidr              = var.vpc_cidr
  enable_nat_per_az     = false  # Single NAT for cost
  enable_vpc_flow_logs  = false  # Dev doesn't need logs
  
  project_name = "stellar-insights"

  tags = {
    Environment = var.environment
  }
}

# Outputs for reference
output "vpc_id" {
  description = "VPC ID"
  value       = module.networking.vpc_id
}

output "public_subnet_ids" {
  description = "Public subnet IDs for ALB"
  value       = module.networking.public_subnet_ids
}

output "private_app_subnet_ids" {
  description = "Private subnet IDs for ECS"
  value       = module.networking.private_app_subnet_ids
}

output "availability_zones" {
  description = "Availability zones used"
  value       = module.networking.availability_zones
}
