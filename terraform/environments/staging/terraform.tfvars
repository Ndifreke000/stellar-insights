# Staging Environment Variables
# Cost estimate: ~$96/month (see module.compute's cost_estimate output)
#
# Breakdown:
#   - NAT Gateway: ~$30/month (1 for cost efficiency)
#   - ALB: ~$20/month
#   - ECS (t3.small): ~$20/month
#   - EFS (SQLite volume): ~$1/month -- no RDS, see docs/adr/0001-sqlite-vs-postgres.md
#   - ElastiCache Redis (cache.t3.small, single node): ~$20/month
#   - Data transfer out: ~$10/month
#
# ✓ Adequate for testing and integration testing
# ✓ Litestream continuous S3 replication + backup.rs local snapshots (see docs/backup-system.md)
# ✓ CloudWatch monitoring enabled
# ✓ 2 AZs for networking/caching (backend is a single pinned instance -- see ADR 0001)

aws_region  = "us-east-1"
environment = "staging"
vpc_cidr    = "10.1.0.0/16"

# Next steps:
# 1. Ensure terraform/global/ has been applied (S3 state bucket, DynamoDB locks)
# 2. Run: terraform init -backend-config="bucket=payraider-terraform-state-$(aws sts get-caller-identity --query Account --output text)"
# 3. Run: terraform plan
# 4. Run: terraform apply
#
# When complete:
# - Configure VAULT_ADDR in GitHub Actions
# - Run Vault setup scripts: scripts/setup-vault-complete.sh
# - Test: exec into the running task and check /data/payraider.db exists
# - Verify the litestream sidecar is replicating: check its CloudWatch logs
