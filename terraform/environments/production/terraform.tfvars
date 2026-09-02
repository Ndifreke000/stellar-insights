# Production Environment Variables
# Cost estimate: ~$142/month (see module.compute's cost_estimate output)
#
# Breakdown:
#   - NAT Gateways: ~$90/month (3 total, 1 per AZ for HA)
#   - ALB: ~$20/month
#   - ECS (t3.small, 1 pinned instance): ~$16/month
#   - EFS (SQLite volume): ~$1/month -- no RDS, see docs/adr/0001-sqlite-vs-postgres.md
#   - ElastiCache Redis (cache.t3.small, Multi-AZ): ~$40/month
#   - Data transfer out: ~$20/month
#
# ✓ Full high availability for networking/Redis (3 AZs)
# ✓ Multi-AZ Redis with automatic failover
# ✓ Litestream continuous S3 replication + backup.rs local snapshots
#   (see docs/backup-system.md)
# ✓ CloudWatch monitoring and alarms
# ✓ VPC Flow Logs for security and troubleshooting
# ✗ No auto-scaling, no backend HA: pinned to 1 task. SQLite permits
#   exactly one writer and there's no shared storage for multiple
#   replicas to safely share the database file -- see ADR 0001.
#
# IMPORTANT: This is a PRODUCTION environment
# - All changes require code review and approval
# - All deployments via GitHub Actions CI/CD
# - NO manual terraform apply on production
# - All database changes must be tested in staging first

aws_region  = "us-east-1"
environment = "production"
vpc_cidr    = "10.2.0.0/16"

# Pre-deployment Checklist:
# [ ] All Vault secrets configured (DATABASE_URL, JWT_SECRET, OAuth credentials, etc)
# [ ] SSL/TLS certificates in ACM for domain
# [ ] Route53 DNS records configured and tested
# [ ] Litestream replication and restore tested in staging (see docs/backup-system.md)
# [ ] CloudWatch alarms configured and tested
# [ ] GitHub Actions variable secrets in place
# [ ] Zapier webhooks registered and tested
# [ ] Load test completed: min 100 req/sec
# [ ] Spike test completed: 10x traffic surge handling
#
# Post-deployment Validation:
# [ ] Health check: GET /health returning 200 OK
# [ ] Database connectivity verified
# [ ] Vault secrets accessible
# [ ] CloudWatch logs flowing
# [ ] Alerts configured and tested (intentional spike)
# [ ] Logging and monitoring dashboards active
# [ ] Runbook reviewed by on-call team
