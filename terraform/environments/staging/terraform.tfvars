# Staging Environment Variables
# Cost estimate: $150-200/month
#
# Breakdown:
#   - NAT Gateway: ~$30/month (1 for cost efficiency)
#   - ALB: ~$20/month
#   - ECS (t3.small): ~$30/month
#   - RDS PostgreSQL (db.t3.small, 100GB): ~$60/month
#   - ElastiCache Redis (cache.t3.small, single node): ~$20/month
#   - Data transfer out: ~$20-30/month
#
# ✓ Adequate for testing and integration testing
# ✓ Automatic RDS backups (7 days retention)
# ✓ CloudWatch monitoring enabled
# ✓ 2 AZs (not fully redundant, cost-optimized)

aws_region  = "us-east-1"
environment = "staging"
vpc_cidr    = "10.1.0.0/16"

# Next steps:
# 1. Ensure terraform/global/ has been applied (S3 state bucket, DynamoDB locks)
# 2. Run: terraform init -backend-config="bucket=stellar-insights-terraform-state-$(aws sts get-caller-identity --query Account --output text)"
# 3. Run: terraform plan
# 4. Run: terraform apply
#
# When complete:
# - Note RDS endpoint from outputs
# - Configure VAULT_ADDR in GitHub Actions
# - Run Vault setup scripts: scripts/setup-vault-complete.sh
# - Test: psql -h <rds_endpoint> -U postgres -d stellar_insights
