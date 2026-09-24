# Disaster Recovery Plan

This document defines the disaster recovery (DR) procedures for PayRaider infrastructure, including recovery objectives, failure scenarios, and detailed runbooks for incident response.

## Executive Summary

**Document Owner**: DevOps Team  
**Last Updated**: 2024-01-19  
**Review Frequency**: Quarterly  
**Testing Frequency**: Monthly restore drill  
**Approval Status**: Ready for implementation

The PayRaider infrastructure is designed for high availability with automatic failover for most components. This plan documents manual recovery procedures for scenarios that cannot be handled automatically.

## Recovery Objectives

### Recovery Time Objective (RTO)

**RTO is the maximum acceptable downtime after a disaster.**

| Failure Scenario | RTO | Notes |
|---|---|---|
| Single server failure | 5 min | Auto-healing via ECS/ALB |
| Single AZ outage | 10 min | Multi-AZ failover automatic |
| Database failure | 5-10 min | Litestream restore onto the EFS volume |
| Entire region loss | 30 min | Manual re-provisioning |
| Data corruption | 15 min | Restore from clean backup |
| Application code loss | 5 min | Redeploy from ECR + GitHub |

### Recovery Point Objective (RPO)

**RPO is the maximum acceptable data loss (time since last backup).**

| Failure Scenario | RPO | Backup Frequency | Notes |
|---|---|---|---|
| Database loss | Seconds | Litestream continuous S3 replication | Point-in-time recovery; see docs/backup-system.md |
| Accidental data deletion | 7 days | Daily snapshots + weekly + monthly | Can restore to point-in-time |
| Ransomware/corruption | 30 days | Immutable monthly backups | S3 versioning enabled |
| Long-term compliance | 180 days | Monthly snapshots | Archived to Glacier after 90 days |

**Target**: 99.9% uptime (8.76 hours/month maximum downtime)

## Infrastructure Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Route53 (DNS)                           │
│              api.payraider.com                       │
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
    ┌───▼──────────────────────────────────┐
    │ ECS (1 pinned task)                   │
    │ • payraider container                 │
    │ • litestream sidecar                  │   No horizontal scaling --
    └───┬────────────────────────────────────┘   see ADR 0001
        │
    ┌───▼────────────────────────────────────┐
    │ Shared/Regional Resources              │
    ├─────────────────────────────────────────┤
    │ • EFS volume (SQLite database file)    │
    │ • ElastiCache Redis (Multi-AZ)         │
    │ • Secrets in Vault                     │
    └────────────────────────────────────────┘
        │
        ▼
    ┌───────────────────────────┐
    │ S3 Backup Bucket          │
    │ • Litestream replica      │
    │   (continuous, WAL-level) │
    │ • Application logs        │
    └───────────────────────────┘
```

There is deliberately no database-tier failover here: SQLite permits exactly
one writer, so the backend runs as a single pinned task rather than the
3-AZ/N-instance topology the diagram above used to show for RDS. See
`docs/adr/0001-sqlite-vs-postgres.md` for why, and `docs/backup-system.md`
for the full backup/restore story.

## Failure Scenarios & Runbooks

### Scenario 1: Database Unavailable (Cannot Connect)

**Duration**: Minutes  
**Scope**: Users cannot query data  
**Cause Examples**: EFS mount failure, corrupted SQLite file, task crash-looping

There's no separate database instance to fail independently of the backend
here -- "database unavailable" means either the single backend task is
down, or it's up but can't read `/data/payraider.db` (EFS mount issue or a
corrupted file). See `docs/adr/0001-sqlite-vs-postgres.md` for why there's
one pinned task instead of a Multi-AZ instance with failover.

#### Detection

```bash
# Health check will fail
curl https://api.payraider.com/health
# Returns 503 Service Unavailable

# Application logs show database errors
aws logs tail /ecs/payraider-production --follow | grep "database\|sqlite\|SQLITE"
```

#### Runbook: Database Recovery

**Time to Execute**: 5-10 minutes  
**Requires**: AWS console access, ECS exec access

**Step 1: Determine if it's the task or the volume** (1 min)
```bash
# Is the task even running?
aws ecs describe-services \
  --cluster payraider-production --services payraider-service \
  --query 'services[0].deployments'

# If a task is running, exec in and check the file directly
aws ecs execute-command --cluster payraider-production \
  --task <task-id> --container payraider --interactive \
  --command "/bin/sh -c 'ls -la /data/ && sqlite3 /data/payraider.db \"PRAGMA integrity_check;\"'"
```

**Step 2a: If the task is crash-looping (not an EFS/file issue)** (2-5 min)
```bash
# Check recent task stop reasons
aws ecs describe-tasks --cluster payraider-production --tasks <task-id> \
  --query 'tasks[0].[stoppedReason,containers[].reason]'

# Force a fresh deployment
aws ecs update-service --cluster payraider-production \
  --service payraider-service --force-new-deployment
```

**Step 2b: If `/data/payraider.db` is missing or fails `PRAGMA integrity_check`** (5-8 min)

Restore the latest Litestream replica onto the volume. See
`docs/backup-system.md` for the full command reference; summary:

```bash
# 1. Restore the latest replica to a local file
litestream restore -o /tmp/restored.db \
  s3://payraider-db-backups-<account>/production/payraider.db

# 2. Stop the task (Recreate deployment strategy means there's only ever
#    one, so this is a brief outage, not a failover)
aws ecs update-service --cluster payraider-production \
  --service payraider-service --desired-count 0

# 3. Copy the restored file onto the EFS volume (via a one-off task with
#    the same volume mounted, or an EFS access point mounted locally)
#    then restart
aws ecs update-service --cluster payraider-production \
  --service payraider-service --desired-count 1
```

**Step 3: Verify** (2 min)
```bash
curl -I https://api.payraider.com/health
# Should return 200 OK
aws logs tail /ecs/payraider-production --follow
```

**Escalation**:
- If the EFS mount itself is broken (not just the file), escalate to
  whoever owns the Terraform (`terraform/modules/compute/ecs/main.tf`'s
  `aws_efs_mount_target` resources) -- this is an infra-level failure, not
  a data one
- If the Litestream replica is also unusable, restore from `backup.rs`'s
  local snapshots instead (see `docs/backup-system.md`, accepting more
  data loss)
- Contact: ops-critical@payraider.com

---

### Scenario 2: Data Corruption (Accidental Deletion/Update)

**Duration**: Hours to days  
**Scope**: Application data is corrupted but the task is running  
**Cause Examples**: Bug in migration, accidental DELETE without WHERE

#### Detection

```bash
# Alert monitoring might notice unusual activity, or discovered via user report

# Check what changed recently (exec into the running task)
sqlite3 /data/payraider.db "SELECT * FROM users WHERE updated_at > datetime('now', '-1 hour') ORDER BY updated_at DESC;"

# Verify database integrity
sqlite3 /data/payraider.db "PRAGMA integrity_check;"
```

#### Runbook: Point-in-Time Restore from Litestream

**Time to Execute**: 10-15 minutes  
**Data Loss**: Seconds to minutes (Litestream replicates continuously, not
on a daily schedule -- restore to just before the bad write)

**Step 1: Freeze Application** (1 min)
```bash
# Stop the task to prevent further writes
aws ecs update-service --cluster payraider-production \
  --service payraider-service --desired-count 0
```

**Step 2: Restore to a point in time before the corruption** (5 min)
```bash
# Estimate: if corruption was detected at 14:30, restore to 14:15
litestream restore -o /tmp/restored.db \
  -timestamp 2026-09-01T14:15:00Z \
  s3://payraider-db-backups-<account>/production/payraider.db
```

**Step 3: Verify Data Integrity** (2 min)
```bash
sqlite3 /tmp/restored.db "SELECT COUNT(*) FROM users;"
sqlite3 /tmp/restored.db "PRAGMA integrity_check;"
```

**Step 4: Copy the restored file onto the EFS volume and restart** (3 min)

Via a one-off task with the same EFS volume mounted (or an EFS access
point mounted locally), copy `/tmp/restored.db` to `/data/payraider.db`,
then:

```bash
aws ecs update-service --cluster payraider-production \
  --service payraider-service --desired-count 1

sleep 30
curl -I https://api.payraider.com/health
```

**Escalation**:
- If the Litestream replica is also corrupted for the relevant window,
  fall back to `backup.rs`'s local snapshots (see `docs/backup-system.md`)
  -- coarser granularity (daily), so expect more data loss
- If corruption is suspected to be malicious rather than a bug: escalate
  to the security team before restoring, to preserve evidence
- Contact: ops-critical@payraider.com

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
curl https://api.payraider.com/health
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
  -target=module.caching \
  -target=module.compute  # includes the EFS volume -- see terraform/modules/compute/ecs/main.tf

# Note: This assumes Terraform state is accessible
# (State bucket should be in separate region or cross-region replicated)

# Option B: Manual provision via AWS console
# Create VPC → subnets → security groups → EFS → ECS cluster
# (Much slower, only as last resort)
```

**Step 3: Restore Database from Backup** (5-10 min)
```bash
# The Litestream S3 bucket (terraform/global/backups.tf) is regional and
# NOT cross-region replicated by default -- if the whole region hosting it
# is down, this bucket is unreachable too. Restore onto the new EFS volume
# from whatever the most recent accessible source is:

# If the backups bucket's region is still reachable:
litestream restore -o /tmp/restored.db \
  s3://payraider-db-backups-<account>/production/payraider.db

# If not, fall back to the last local snapshot pulled out of backup.rs's
# BACKUP_DIR before the region went down (see docs/backup-system.md) --
# expect more data loss in this path

# Then copy /tmp/restored.db onto the new region's EFS volume before
# starting the ECS service.
```

**Step 4: Redeploy Application** (5 min)
```bash
# Pull latest image from ECR (assume cross-region replicated)
aws ecr get-images --repository-name payraider-backend --region us-west-2

# Deploy to new ECS cluster (desired-count 1 -- see ADR 0001, no horizontal scaling)
aws ecs create-service \
  --cluster payraider-production-us-west-2 \
  --service-name payraider-service \
  --task-definition payraider-production \
  --desired-count 1 \
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
        "Name": "api.payraider.com",
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
nslookup api.payraider.com
# Should resolve to us-west-2 endpoint
```

**Step 6: Validation & Monitoring** (5 min)
```bash
# Test application
curl https://api.payraider.com/health
# Should return 200 OK

# Monitor logs
aws logs tail /ecs/payraider-production-us-west-2 --follow

# Check the database file is present and intact (exec into the task)
sqlite3 /data/payraider.db "SELECT COUNT(*) FROM users;"
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
- Contact: ops-critical@payraider.com, infrastructure@payraider.com

---

### Scenario 4: Application Failure (Cannot Deploy)

**Duration**: Minutes  
**Scope**: Application won't start or crashes immediately  
**Cause Examples**: Bad deployment, dependency missing, configuration error

#### Detection

```bash
# ECS tasks fail to start or keep restarting
aws ecs describe-tasks \
  --cluster payraider-production \
  --tasks $(aws ecs list-tasks --cluster payraider-production --service-name payraider-service --query taskArns --output text) \
  --query 'tasks[*].[taskArn, lastStatus, stoppedReason]' \
  --output table

# Health checks failing
curl https://api.payraider.com/health
# Returns 503 or connection refused
```

#### Runbook: Rollback Deployment

**Time to Execute**: 5 minutes  
**Requires**: Previous working task definition

**Step 1: Stop Bad Deployment** (1 min)
```bash
# Scale down broken tasks
aws ecs update-service \
  --cluster payraider-production \
  --service payraider-service \
  --desired-count 0

echo "✓ Bad deployment stopped"
```

**Step 2: Find Last Working Task Definition** (1 min)
```bash
# List recent task definitions (newest first)
aws ecs describe-task-definition \
  --task-definition payraider-production \
  --query 'taskDefinition.taskDefinitionArn' \
  --output text

# Get specific revision
PREV_REVISION=$(aws ecs list-task-definitions \
  --family-prefix payraider-production \
  --sort DESC \
  --query 'taskDefinitionArns[1]' \
  --output text)

echo "Previous task definition: $PREV_REVISION"
```

**Step 3: Rollback to Previous Version** (2 min)
```bash
# Update service to use previous task definition
aws ecs update-service \
  --cluster payraider-production \
  --service payraider-service \
  --task-definition "$PREV_REVISION" \
  --desired-count 3

echo "✓ Rolled back to $PREV_REVISION"
```

**Step 4: Verify Rollback** (2 min)
```bash
# Wait for tasks to start
sleep 30

# Check health
curl https://api.payraider.com/health
# Should return 200 OK

# Monitor logs
aws logs tail /ecs/payraider-production --follow
```

**Step 5: Investigate Root Cause** (offline)
```bash
# Review deployment logs
gh run view $(gh run list -w deploy.yml --limit 1 --json databaseId | jq -r '.[]') --log-status all

# Check code changes
git log -1 --oneline

# Verify Docker image in ECR
aws ecr describe-images \
  --repository-name payraider-backend \
  --query 'imageDetails[-1].[imageTags, imageSizeInBytes, imagePushedAt]'
```

**Escalation**:
- For quick rollback: DevOps team has permission
- For investigation: Development team + DevOps
- Contact: ops-critical@payraider.com

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
   # - ops-critical@payraider.com
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
- PagerDuty: https://payraider.pagerduty.com/
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
# Litestream (see docs/backup-system.md for the full reference)
litestream restore -o ./restored.db s3://payraider-db-backups-<account>/production/payraider.db
litestream restore -o ./restored.db -timestamp <RFC3339> s3://payraider-db-backups-<account>/production/payraider.db

# ECS operations
aws ecs describe-services --cluster payraider-production --services payraider-service
aws ecs update-service --cluster payraider-production --service payraider-service --desired-count 1
aws ecs list-tasks --cluster payraider-production
aws ecs describe-tasks --cluster payraider-production --tasks <task-arn>
aws ecs execute-command --cluster payraider-production --task <task-id> --container payraider --interactive --command "/bin/sh"

# Route53 DNS
aws route53 list-hosted-zones
aws route53 list-resource-record-sets --hosted-zone-id Z1234567890ABC
aws route53 change-resource-record-sets --hosted-zone-id Z1234567890ABC --change-batch file://change.json

# S3 (Litestream replica + backup.rs local-snapshot uploads, if configured)
aws s3 ls s3://payraider-db-backups-<account>/production/ --recursive
```

### SQLite Commands

```bash
# Connect to database (exec into the running task, or use the restored file locally)
sqlite3 /data/payraider.db

# Check table sizes (SQLite has no pg_total_relation_size equivalent;
# dbstat is the closest built-in)
SELECT name, SUM(pgsize) AS bytes FROM dbstat GROUP BY name ORDER BY bytes DESC;

# Verify specific table
SELECT COUNT(*) as row_count, MAX(updated_at) as last_update FROM users;

# Integrity check
sqlite3 /data/payraider.db "PRAGMA integrity_check;"

# Manual backup (see docs/backup-system.md for what backup.rs already
# does on a schedule -- this is the ad hoc version)
sqlite3 /data/payraider.db ".backup /tmp/manual-backup.db"
gzip /tmp/manual-backup.db
```

## Revision History

| Version | Date | Change | Author |
|---|---|---|---|
| 1.0 | 2024-01-19 | Initial DR plan created | DevOps Team |

---

**Document Classification**: Internal  
**Last Review Date**: 2024-01-19  
**Next Review Date**: 2024-04-19  
**Owner**: DevOps Team (ops-critical@payraider.com)

This document is version-controlled in the GitHub repository.  
To suggest changes, open a PR or file an issue.
