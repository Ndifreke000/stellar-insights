# Disaster Recovery Plan

This document defines the disaster recovery (DR) procedures for Stellar Insights infrastructure, including recovery objectives, failure scenarios, and detailed runbooks for incident response.

## Executive Summary

**Document Owner**: DevOps Team  
**Last Updated**: 2024-01-19  
**Review Frequency**: Quarterly  
**Testing Frequency**: Monthly restore drill  
**Approval Status**: Ready for implementation

The Stellar Insights infrastructure is designed for high availability with automatic failover for most components. This plan documents manual recovery procedures for scenarios that cannot be handled automatically.

## Recovery Objectives

### Recovery Time Objective (RTO)

**RTO is the maximum acceptable downtime after a disaster.**

| Failure Scenario | RTO | Notes |
|---|---|---|
| Single server failure | 5 min | Auto-healing via ECS/ALB |
| Single AZ outage | 10 min | Multi-AZ failover automatic |
| Database failure | 10 min | Restore from automated snapshot |
| Entire region loss | 30 min | Manual re-provisioning |
| Data corruption | 15 min | Restore from clean backup |
| Application code loss | 5 min | Redeploy from ECR + GitHub |

### Recovery Point Objective (RPO)

**RPO is the maximum acceptable data loss (time since last backup).**

| Failure Scenario | RPO | Backup Frequency | Notes |
|---|---|---|---|
| Database loss | 24 hours | Daily automated snapshot | RDS backups are continuous |
| Accidental data deletion | 7 days | Daily snapshots + weekly + monthly | Can restore to point-in-time |
| Ransomware/corruption | 30 days | Immutable monthly backups | S3 versioning enabled |
| Long-term compliance | 180 days | Monthly snapshots | Archived to Glacier after 90 days |

**Target**: 99.9% uptime (8.76 hours/month maximum downtime)

## Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Route53 (DNS)                           │
│              api.stellar-insights.com                       │
└──────────────────────┬──────────────────────────────────────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
    ┌───▼──┐      ┌────▼─┐      ┌───▼──┐
    │ AZ-A │      │ AZ-B │      │ AZ-C │   (3 Availability Zones)
    │      │      │      │      │      │   Multi-AZ failover
    └───┬──┘      └────┬─┘      └───┬──┘
        │              │              │
    ┌───▼──────────────▼──────────────▼──────┐
    │  Application Load Balancer (ALB)       │
    │  • HTTPS termination                   │
    │  • Health check every 30s               │
    │  • Blue-green deployment ready         │
    └───┬──────────────┬──────────────┬──────┘
        │              │              │
    ┌───▼───┐      ┌───▼───┐      ┌──▼───┐
    │ECS-A  │      │ECS-B  │      │ECS-C │   (3+ ECS tasks)
    │       │      │       │      │      │   Auto-scaling 3-10
    └───┬───┘      └───┬───┘      └──┬───┘
        │              │              │
    ┌───▼──────────────▼──────────────▼──────┐
    │ Shared Resources (Multi-AZ)            │
    ├────────────────────────────────────────┤
    │ • RDS PostgreSQL (primary)             │
    │ • RDS PostgreSQL (standby replica)     │
    │ • ElastiCache Redis                    │
    │ • EBS volumes (encrypted)              │
    │ • Secrets in AWS Secrets Manager       │
    │ • Configuration in Parameter Store     │
    └────────────────────────────────────────┘
        │              │
        ▼              ▼
    ┌───────────────────────────┐
    │ S3 Backup Buckets         │
    │ • Database snapshots      │
    │ • Parquet exports         │
    │ • Application logs        │
    │ • Disaster recovery data  │
    └───────────────────────────┘
```

## Failure Scenarios & Runbooks

### Scenario 1: Database Unavailable (Cannot Connect)

**Duration**: Minutes  
**Scope**: Users cannot query data  
**Cause Examples**: Crashed instance, network misconfiguration, security group mismatch

#### Detection

```bash
# Health check will fail
curl https://api.stellar-insights.com/health
# Returns 503 Service Unavailable

# Application logs show database connection errors
aws logs tail /ecs/stellar-insights-production --follow | grep "database\|psql\|connection"
```

#### Runbook: Database Recovery

**Time to Execute**: 10-15 minutes  
**Requires**: AWS console access + psql CLI

**Step 1: Verify Database Status** (1 min)
```bash
# Check instance status
aws rds describe-db-instances \
  --db-instance-identifier stellar-insights-production \
  --query 'DBInstances[0].[DBInstanceStatus, DBInstanceClass, MultiAZ]' \
  --output table

# If status is "available", check connectivity
psql -h stellar-insights-production.c0xxxxxxxxxxxx.us-east-1.rds.amazonaws.com \
  -U postgres -d stellar_insights -c "SELECT 1;" --username postgres
# If this hangs, network connectivity is broken
```

**Step 2: Check Recent Events** (1 min)
```bash
# View RDS events to understand what happened
aws rds describe-events \
  --source-type db-instance \
  --source-identifier stellar-insights-production \
  --query 'Events[0:10].[EventCategories, Message, SourceType, SourceIdentifier]' \
  --output table
```

**Step 3: Restore from Snapshot if Instance Unrecoverable** (8-10 min)
```bash
# If instance cannot be recovered, restore from latest snapshot

# 1. Find latest good snapshot
LATEST_SNAPSHOT=$(aws rds describe-db-snapshots \
  --db-instance-identifier stellar-insights-production \
  --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[-1].DBSnapshotIdentifier' \
  --output text)

echo "Latest snapshot: $LATEST_SNAPSHOT"

# 2. Create new instance from snapshot
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier stellar-insights-production-restore-$(date +%s) \
  --db-snapshot-identifier "$LATEST_SNAPSHOT" \
  --db-instance-class db.t3.medium \
  --multi-az \
  --no-publicly-accessible \
  --vpc-security-group-ids sg-xxxxxxxx \
  --db-subnet-group-name stellar-insights-db-production

# 3. Wait for instance to be available (5-10 minutes)
NEW_INSTANCE="stellar-insights-production-restore-$(date +%s)"
aws rds wait db-instance-available --db-instance-identifier "$NEW_INSTANCE"

# 4. Test new instance
NEW_ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier "$NEW_INSTANCE" \
  --query 'DBInstances[0].Endpoint.Address' \
  --output text)

psql -h "$NEW_ENDPOINT" -U postgres -d stellar_insights -c "SELECT COUNT(*) FROM users;"
```

**Step 4: Switch Application Traffic** (2 min)
```bash
# Option A: Update RDS endpoint in Secrets Manager
aws secretsmanager update-secret \
  --secret-id "stellar-insights/production/database" \
  --secret-string '{"host":"'$NEW_ENDPOINT'","username":"postgres",...}'

# Option B: Update Route53 CNAME (if using custom domain)
aws route53 change-resource-record-sets \
  --hosted-zone-id Z1234567890ABC \
  --change-batch '{
    "Changes": [{
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "db.stellar-insights.com",
        "Type": "CNAME",
        "TTL": 60,
        "ResourceRecords": [{"Value": "'$NEW_ENDPOINT'"}]
      }
    }]
  }'

# Force application to reconnect
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --force-new-deployment
```

**Step 5: Monitor Recovery** (5 min)
```bash
# Watch ECS tasks reconnect to new database
aws ecs describe-services \
  --cluster stellar-insights-production \
  --services stellar-insights-service \
  --query 'Services[0].DeploymentConfiguration' \
  --output table

# Check application health
curl -I https://api.stellar-insights.com/health
# Should return 200 OK

# Monitor logs for errors
aws logs tail /ecs/stellar-insights-production --follow
```

**Step 6: Cleanup** (2 min)
```bash
# Once restored instance is confirmed healthy, delete old instance
aws rds delete-db-instance \
  --db-instance-identifier stellar-insights-production \
  --create-final-snapshot \
  --final-db-snapshot-identifier stellar-insights-production-final-$(date +%s)

# Rename restored instance to original name
# (AWS doesn't support rename, so DNS update was done above)
```

**Escalation**:
- If restore fails, page on-call DBA and escalate to AWS support
- If multiple snapshots are corrupted, restore from weekly or monthly backup
- Contact: ops-critical@stellar-insights.com

---

### Scenario 2: Data Corruption (Accidental Deletion/Update)

**Duration**: Hours to days  
**Scope**: Application data is corrupted but instance is running  
**Cause Examples**: Bug in migration, accidental DELETE without WHERE, ransomware

#### Detection

```bash
# Alert monitoring might notice unusual activity
# Or discovered via user report

# Check what changed recently
SELECT * FROM users WHERE updated_at > NOW() - INTERVAL '1 hour' ORDER BY updated_at DESC;

# Verify table integrity
PRAGMA integrity_check;  # SQLite
CHECK TABLE table_name;  # MySQL
# PostgreSQL doesn't have built-in check, use pg_dump to validate
```

#### Runbook: Restore from Clean Backup

**Time to Execute**: 15-20 minutes  
**Data Loss**: Maximum 24 hours (last daily backup)

**Step 1: Freeze Application** (1 min)
```bash
# Stop accepting writes to prevent further corruption
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --desired-count 0

echo "✓ Application traffic stopped"

# Wait for existing connections to drain (30s)
sleep 30
```

**Step 2: Assess Damage** (2 min)
```bash
# Quick query to see extent of corruption
psql -h stellar-insights-production.c0xxxx.us-east-1.rds.amazonaws.com \
  -U postgres -d stellar_insights <<EOF
SELECT relname, n_live_tup, n_dead_tup 
FROM pg_stat_user_tables 
ORDER BY n_dead_tup DESC;
EOF

# Check backup retention period
aws rds describe-db-instances \
  --db-instance-identifier stellar-insights-production \
  --query 'DBInstances[0].BackupRetentionPeriod' \
  --output text
```

**Step 3: Create New Instance from Backup Before Corruption** (8-10 min)
```bash
# Point-in-time recovery: restore to a time BEFORE corruption occurred
# Estimate: if corruption was detected at 14:30, restore to 14:15

RESTORE_TIME="2024-01-19T14:15:00Z"  # Time before corruption

aws rds restore-db-instance-to-point-in-time \
  --source-db-instance-identifier stellar-insights-production \
  --target-db-instance-identifier stellar-insights-production-pitr-$(date +%s) \
  --restore-time "$RESTORE_TIME" \
  --db-instance-class db.t3.medium \
  --multi-az \
  --no-publicly-accessible

# Wait for recovery
aws rds wait db-instance-available \
  --db-instance-identifier "stellar-insights-production-pitr-$(date +%s)"
```

**Step 4: Verify Data Integrity** (2 min)
```bash
PITR_INSTANCE="stellar-insights-production-pitr-$(date +%s)"

# Check specific corrupted table
PITR_ENDPOINT=$(aws rds describe-db-instances \
  --db-instance-identifier "$PITR_INSTANCE" \
  --query 'DBInstances[0].Endpoint.Address' --output text)

psql -h "$PITR_ENDPOINT" -U postgres -d stellar_insights <<EOF
SELECT COUNT(*) as user_count FROM users;
SELECT COUNT(*) as corrupted_records FROM users WHERE status = 'CORRUPTED';
EOF
```

**Step 5: Switch Application to Restored Instance** (3 min)
```bash
# Update secrets manager
aws secretsmanager update-secret \
  --secret-id "stellar-insights/production/database" \
  --secret-string '{"host":"'$PITR_ENDPOINT'","username":"postgres",...}'

# Restart application with new endpoint
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --desired-count 3  # Restart 3 tasks

# Verify health
sleep 30
curl -I https://api.stellar-insights.com/health
```

**Step 6: Cleanup** (2 min)
```bash
# Delete corrupted original instance
aws rds delete-db-instance \
  --db-instance-identifier stellar-insights-production \
  --skip-final-snapshot

# Rename PITR instance to original name
# (Done via DNS/Secrets Manager, AWS doesn't support rename)
```

**Escalation**:
- If restore fails: Page on-call DBA
- If multiple backups corrupted: Escalate to AWS support + security team (possible ransomware)
- Contact: ops-critical@stellar-insights.com

---

### Scenario 3: Region Outage (AWS Region Unavailable)

**Duration**: Minutes to hours  
**Scope**: Entire infrastructure in region is down  
**Cause Examples**: AWS data center failure, regional network partition

#### Detection

```bash
# Health checks will fail from multiple regions
# AWS status page: https://status.aws.amazon.com/

# Application monitoring shows 100% error rate
curl https://api.stellar-insights.com/health
# Connection timeout or refused
```

#### Runbook: Regional Failover

**Time to Execute**: 30-45 minutes  
**Requires**: Pre-configured standby region (if using multi-region setup)  
**Data Loss**: Potential minute-level loss from last backup

**Note**: Current single-region design requires manual re-provisioning. For critical environments, consider:
- Multi-region active-passive setup (higher cost)
- Automated failover via Route53 health checks + Lambda

**Step 1: Declare Regional Disaster** (1 min)
```bash
# Notify stakeholders
# Trigger incident commander page
# Start war room call (see escalation section below)

# Verify region is actually down
aws ec2 describe-instances --region us-east-1 2>&1 | grep -i "unavailable\|connection"
```

**Step 2: Provision New Infrastructure in Alternate Region** (20-30 min)
```bash
# Option A: Use Terraform to provision in us-west-2
cd terraform/environments/production

# Override region
terraform apply \
  -var aws_region=us-west-2 \
  -var vpc_cidr=10.4.0.0/16 \
  -target=module.networking \
  -target=module.database \
  -target=module.caching

# Note: This assumes Terraform state is accessible
# (State bucket should be in separate region or cross-region replicated)

# Option B: Manual provision via AWS console
# Create VPC → subnets → security groups → RDS → ECS cluster
# (Much slower, only as last resort)
```

**Step 3: Restore Database from Backup** (5-10 min)
```bash
# Restore from latest backup in S3 (cross-region)
# Snapshots in original region are inaccessible

# Download Parquet export from S3 (in alternate region)
aws s3 cp s3://stellar-insights-backups-replica/database/production/exports/latest.parquet . \
  --region us-west-2

# Restore into new PostgreSQL instance
pg_restore --host new-rds-endpoint --user postgres --database stellar_insights latest.parquet
```

**Step 4: Redeploy Application** (5 min)
```bash
# Pull latest image from ECR (assume cross-region replicated)
aws ecr get-images --repository-name stellar-insights-backend --region us-west-2

# Deploy to new ECS cluster
aws ecs create-service \
  --cluster stellar-insights-production-us-west-2 \
  --service-name stellar-insights-service \
  --task-definition stellar-insights-production \
  --desired-count 3 \
  --region us-west-2
```

**Step 5: Update DNS** (1 min)
```bash
# Update Route53 to point to new region
aws route53 change-resource-record-sets \
  --hosted-zone-id Z1234567890ABC \
  --change-batch '{
    "Changes": [{
      "Action": "UPSERT",
      "ResourceRecordSet": {
        "Name": "api.stellar-insights.com",
        "Type": "A",
        "AliasTarget": {
          "HostedZoneId": "Z3AQMSTJ2NG68D",
          "DNSName": "prod-alb-us-west-2.elb.us-west-2.amazonaws.com",
          "EvaluateTargetHealth": true
        }
      }
    }]
  }'

# Verify DNS propagation
nslookup api.stellar-insights.com
# Should resolve to us-west-2 endpoint
```

**Step 6: Validation & Monitoring** (5 min)
```bash
# Test application
curl https://api.stellar-insights.com/health
# Should return 200 OK

# Monitor logs
aws logs tail /ecs/stellar-insights-production-us-west-2 --follow

# Check database connectivity
psql -h new-rds-endpoint -U postgres -d stellar_insights -c "SELECT COUNT(*) FROM users;"
```

**Step 7: Failback (When Original Region Recovers)** (10 min)
```bash
# Once AWS confirms original region is healthy:

# 1. Update DNS back to us-east-1
# 2. Verify application health in original region
# 3. Delete temporary infrastructure in us-west-2
# 4. Restore database to original region from backup
# 5. Update application configuration
```

**Escalation**:
- Declare outage to all stakeholders immediately
- Page on-call infrastructure team + DBA
- Contact AWS support (Enterprise support for fastest response)
- Consider manual traffic diversion while re-provisioning
- Contact: ops-critical@stellar-insights.com, infrastructure@stellar-insights.com

---

### Scenario 4: Application Failure (Cannot Deploy)

**Duration**: Minutes  
**Scope**: Application won't start or crashes immediately  
**Cause Examples**: Bad deployment, dependency missing, configuration error

#### Detection

```bash
# ECS tasks fail to start or keep restarting
aws ecs describe-tasks \
  --cluster stellar-insights-production \
  --tasks $(aws ecs list-tasks --cluster stellar-insights-production --service-name stellar-insights-service --query taskArns --output text) \
  --query 'tasks[*].[taskArn, lastStatus, stoppedReason]' \
  --output table

# Health checks failing
curl https://api.stellar-insights.com/health
# Returns 503 or connection refused
```

#### Runbook: Rollback Deployment

**Time to Execute**: 5 minutes  
**Requires**: Previous working task definition

**Step 1: Stop Bad Deployment** (1 min)
```bash
# Scale down broken tasks
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --desired-count 0

echo "✓ Bad deployment stopped"
```

**Step 2: Find Last Working Task Definition** (1 min)
```bash
# List recent task definitions (newest first)
aws ecs describe-task-definition \
  --task-definition stellar-insights-production \
  --query 'taskDefinition.taskDefinitionArn' \
  --output text

# Get specific revision
PREV_REVISION=$(aws ecs list-task-definitions \
  --family-prefix stellar-insights-production \
  --sort DESC \
  --query 'taskDefinitionArns[1]' \
  --output text)

echo "Previous task definition: $PREV_REVISION"
```

**Step 3: Rollback to Previous Version** (2 min)
```bash
# Update service to use previous task definition
aws ecs update-service \
  --cluster stellar-insights-production \
  --service stellar-insights-service \
  --task-definition "$PREV_REVISION" \
  --desired-count 3

echo "✓ Rolled back to $PREV_REVISION"
```

**Step 4: Verify Rollback** (2 min)
```bash
# Wait for tasks to start
sleep 30

# Check health
curl https://api.stellar-insights.com/health
# Should return 200 OK

# Monitor logs
aws logs tail /ecs/stellar-insights-production --follow
```

**Step 5: Investigate Root Cause** (offline)
```bash
# Review deployment logs
gh run view $(gh run list -w deploy.yml --limit 1 --json databaseId | jq -r '.[]') --log-status all

# Check code changes
git log -1 --oneline

# Verify Docker image in ECR
aws ecr describe-images \
  --repository-name stellar-insights-backend \
  --query 'imageDetails[-1].[imageTags, imageSizeInBytes, imagePushedAt]'
```

**Escalation**:
- For quick rollback: DevOps team has permission
- For investigation: Development team + DevOps
- Contact: ops-critical@stellar-insights.com

---

## Communication & Escalation

### War Room Setup

When major incident occurs:

1. **Declare Incident**
   - Severity level: Critical (any scenario above)
   - Incident commander assigned
   - War room Zoom/Slack channel created

2. **Notify Stakeholders**
   ```bash
   # Send notifications to:
   # - ops-critical@stellar-insights.com
   # - #incidents Slack channel
   # - PagerDuty escalation policy
   # - Executive team (if customer-facing)
   ```

3. **Incident Commander Responsibilities**
   - Coordinate response across teams
   - Keep running status update document
   - Make decision authority for trade-offs (speed vs. correctness)
   - Regular updates to stakeholders (every 15 min)

### Escalation Matrix

| Severity | Response Time | Who to Page | Next Escalation |
|---|---|---|---|
| Critical (Total Outage) | Immediate | On-call Engineer | Incident Commander + Manager |
| Critical (Partial Outage) | 5 min | On-call Engineer | On-call DBA if database |
| High (Degradation) | 15 min | Team Lead | Engineering Manager |
| Medium (Error Spike) | 30 min | Team On-Call | No escalation unless unresolved in 1hr |

### Escalation Contacts

**Primary On-Call**:
- PagerDuty: https://stellar-insights.pagerduty.com/
- Page duty rotation escalates within 5 min if not acknowledged

**Secondary Escalation**:
- Engineering Manager: [To be filled in by team]
- Database Administrator: [To be filled in by team]
- AWS Account Manager: [To be filled in by team]

**Executive Notification**:
- Chief Technology Officer: [To be filled in by team]
- VP Operations: [To be filled in by team]
- (Notify after 30+ minutes of ongoing outage)

## Testing & Drills

### Monthly Restore Drill

**Objective**: Verify backup restore procedures work  
**Frequency**: 1st Monday of every month, 10 AM UTC  
**Duration**: 45 minutes  
**Participants**: 2-3 person team (DBA + DevOps + Eng)

**Procedure**:
1. Select random backup (weekly or monthly)
2. Provision temporary database instance
3. Restore from backup
4. Verify data integrity (row counts, checksums)
5. Document findings and lessons learned
6. Delete temporary instance
7. Report results in postmortem

**Success Criteria**:
- Restore completes in < 20 minutes
- Data integrity verified with zero corruption
- Procedure documentation is accurate

**Failure Action**:
- If restore fails, treat as critical incident
- Page DBA and infrastructure team immediately
- Investigate root cause before signing off

### Quarterly Failover Drill

**Objective**: Practice full region failover  
**Frequency**: 1st Monday of each quarter  
**Duration**: 2-3 hours  
**Participants**: Full DevOps + DBA + Engineering

**Procedure**:
1. Announce planned drill to stakeholders (24hr notice)
2. Create parallel infrastructure in alternate region
3. Restore database from backup
4. Deploy application to alternate region
5. Update DNS to alternate region
6. Test application functionality
7. Failback to original region
8. Run postmortem

**Success Criteria**:
- Failover completes in < 60 minutes
- Zero data loss (within RPO)
- All services operational in alternate region
- Documentation accurately reflects actual procedures

### Chaos Engineering Tests (Optional)

For enhanced readiness, consider:
- Kill random ECS tasks (Gremlin/Chaos Toolkit)
- Simulate database slowness (tc/latency injection)
- Simulate network partition (iptables rules)
- Intentional data corruption scenarios

## Change Management

### Change Advisory Board (CAB)

All infrastructure changes go through CAB:

1. **Plan Phase**: 24 hours notice
   - Document change with justification
   - Identify risk and mitigation
   - Assign change owner
   - Schedule change window

2. **Review Phase**: Peer review
   - Technical review (is approach sound?)
   - Security review (does it create vulnerabilities?)
   - Operations review (can we support it?)

3. **Implementation Phase**: Controlled rollout
   - Implement in staging first
   - Run automated tests
   - Manual validation
   - Gradual rollout (canary/blue-green)

4. **Verification Phase**: Post-change validation
   - Automated monitoring checks
   - Manual smoke tests
   - Performance baselines
   - Security scan

### Blackout Dates

Do NOT perform changes during:
- Major conferences/events where customer traffic spikes
- Friday afternoons or evenings (no support response)
- Before holidays (reduced staff availability)
- During known AWS maintenance windows

## Related Documentation

- [Backup System](backup-system.md) - Backup strategy and retention
- [Terraform Infrastructure](../terraform/README.md) - Infrastructure code and provisioning
- [CI/CD Optimization](CI_CD_OPTIMIZATION.md) - Automated deployment pipeline
- [AWS Well-Architected Framework](https://aws.amazon.com/architecture/well-architected/)
- [AWS Disaster Recovery Solutions](https://aws.amazon.com/disaster-recovery/)

## Appendix: Tool References

### AWS CLI Commands

```bash
# RDS snapshots
aws rds describe-db-snapshots --db-instance-identifier stellar-insights-production
aws rds create-db-snapshot --db-instance-identifier stellar-insights-production --db-snapshot-identifier manual-$(date +%s)
aws rds restore-db-instance-from-db-snapshot --source-db-instance-identifier stellar-insights-production --target-db-instance-identifier restore-$(date +%s)
aws rds restore-db-instance-to-point-in-time --source-db-instance-identifier stellar-insights-production --restore-time 2024-01-19T14:00:00Z

# ECS operations
aws ecs describe-services --cluster stellar-insights-production --services stellar-insights-service
aws ecs update-service --cluster stellar-insights-production --service stellar-insights-service --desired-count 3
aws ecs list-tasks --cluster stellar-insights-production
aws ecs describe-tasks --cluster stellar-insights-production --tasks <task-arn>

# Route53 DNS
aws route53 list-hosted-zones
aws route53 list-resource-record-sets --hosted-zone-id Z1234567890ABC
aws route53 change-resource-record-sets --hosted-zone-id Z1234567890ABC --change-batch file://change.json

# S3 backups
aws s3 ls s3://stellar-insights-backups/database/ --recursive
aws s3 cp s3://stellar-insights-backups/database/production/exports/latest.parquet .
```

### PostgreSQL Commands

```bash
# Connect to database
psql -h <rds-endpoint> -U postgres -d stellar_insights

# Check table sizes
SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) 
FROM pg_tables 
WHERE schemaname NOT IN ('pg_catalog', 'information_schema') 
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

# Verify specific table
SELECT COUNT(*) as row_count, MAX(updated_at) as last_update FROM users;

# Dump database
pg_dump -U postgres -d stellar_insights | gzip > backup.sql.gz

# Restore database
gunzip < backup.sql.gz | psql -U postgres -d stellar_insights
```

## Revision History

| Version | Date | Change | Author |
|---|---|---|---|
| 1.0 | 2024-01-19 | Initial DR plan created | DevOps Team |

---

**Document Classification**: Internal  
**Last Review Date**: 2024-01-19  
**Next Review Date**: 2024-04-19  
**Owner**: DevOps Team (ops-critical@stellar-insights.com)

This document is version-controlled in the GitHub repository.  
To suggest changes, open a PR or file an issue.
