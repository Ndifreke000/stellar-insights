# Development Environment Variables
# Cost estimate: $50-70/month
# 
# Breakdown:
#   - NAT Gateway: ~$30/month (single NAT is cheaper than per-AZ)
#   - ALB: ~$20/month
#   - ECS (t3.micro): ~$5/month
#   - Data transfer out: ~$10/month
#   - RDS: skipped (uses SQLite locally)
#   - ElastiCache: skipped (uses Redis container for dev)

aws_region = "us-east-1"
environment = "dev"

# VPC Configuration
vpc_cidr = "10.0.0.0/16"

# Public subnets for ALB (2 AZs for dev, not 3)
# private app/db subnets scale to match

# ✓ All resources use minimum cost tiers
# ✓ No Multi-AZ RDS (only 1 AZ)
# ✓ No redundant NAT gateways (1 total)
# ✓ Minimal monitoring (CloudWatch disabled)
# ✓ Standard (not enhanced) RDS monitoring
# ✓ No backups beyond 7 days

# Next steps:
# 1. Run: terraform init -backend-config="bucket=stellar-insights-terraform-state-$(aws sts get-caller-identity --query Account --output text)"
# 2. Run: terraform plan
# 3. Review cost: terraform plan | grep -i cost || echo "(run with cost plugin)"
# 4. Run: terraform apply
