# Production Environment Variables
# Cost estimate: $400-600/month
#
# Breakdown:
#   - NAT Gateways: ~$90/month (3 total, 1 per AZ for HA)
#   - ALB: ~$20/month
#   - ECS (t3.small, 3 instances): ~$90/month
#   - RDS PostgreSQL (db.t3.small Multi-AZ, 500GB): ~$150-200/month
#   - ElastiCache Redis (cache.t3.small, Multi-AZ): ~$40/month
#   - Data transfer out: ~$30-50/month
#
# ✓ Full high availability (3 AZs)
# ✓ Multi-AZ RDS with automatic failover
# ✓ Multi-AZ Redis with automatic failover
# ✓ 30-day backup retention for RDS
# ✓ CloudWatch monitoring and alarms
# ✓ VPC Flow Logs for security and troubleshooting
# ✓ Auto-scaling enabled (ECS scale to 2-4 instances)
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
# [ ] RDS backup and restore tested in staging
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
