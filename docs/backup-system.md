# Backup System Documentation

This document describes the automated backup system for Stellar Insights infrastructure, including backup strategy, retention policy, restoration procedures, and disaster recovery integration.

## Overview

The Stellar Insights backup system provides automated daily backups of:

- **Primary Database**: RDS PostgreSQL (multi-AZ with automated backups)
- **Backup Storage**: S3 bucket for long-term retention and off-site storage
- **Retention Policy**: Daily/Weekly/Monthly tiered retention (details below)

**Key Metrics**:
- Backup Frequency: Daily (2 AM UTC)
- Recovery Point Objective (RPO): 24 hours
- Recovery Time Objective (RTO): 10-15 minutes (database restore from snapshot)
- Maximum Retention: 6 months (180 days)

## Backup Strategy

### Database Backups

The backup system creates multiple types of backups:

#### 1. RDS Automated Snapshots (Continuous)

AWS RDS automatically creates snapshots of the database based on the backup retention period:

- **Frequency**: Hourly incremental snapshots
- **Retention**: 30 days (production), 14 days (staging), 7 days (dev)
- **Scope**: Full database backup (point-in-time recovery possible)
- **Storage**: Managed by AWS in S3 (encrypted, replicated across AZs)

**Benefit**: Zero configuration, automatic, point-in-time recovery within retention window.

#### 2. Backup Snapshots (Daily)

Explicit snapshots are created daily via the backup job:

- **Frequency**: Once per day at 2 AM UTC
- **Trigger**: GitHub Actions scheduled workflow (`backup-database.yml`)
- **Naming**: `stellar-insights-{environment}-backup-{YYYYMMDD-HHMMSS}`
- **Tags**: Type, Environment, CreatedBy, Workflow

**Benefit**: Explicit snapshots survive the RDS retention period, enabling long-term retention.

#### 3. S3 Exports (Asynchronous)

Snapshots are exported to Parquet format in S3:

- **Frequency**: Daily (triggered after snapshot completion)
- **Location**: `s3://stellar-insights-backups/database/{environment}/exports/`
- **Format**: Parquet (efficient, queryable, column-oriented)
- **Storage Class**: INTELLIGENT_TIERING (cost-optimized)

**Benefit**: Long-term archival, queryable without full restore, cross-region durability.

### Backup Workflow

```
2 AM UTC (Daily)
  ↓
[GitHub Actions: backup-database.yml]
  ├─ Create RDS manual snapshot
  │   └─ Wait for completion (~10-30 minutes)
  ├─ Initiate S3 export (async, ~10-20 minutes)
  ├─ Verify snapshot in RDS console
  ├─ Delete snapshots older than retention policy
  └─ Report in GitHub Actions summary
```

## Retention Policy

The backup system implements a tiered retention policy balancing storage costs and recovery needs:

| Policy | Duration | Frequency | Scope | Use Case |
|--------|----------|-----------|-------|----------|
| **Daily** | 7 days | Every day | Last 7 backups | Recent data loss, quick recovery |
| **Weekly** | 28 days | Sundays only | 4 weekly snapshots | Last-week restore needs |
| **Monthly** | 180 days | 1st of month | 6 monthly snapshots | Long-term archive, compliance |

### Retention Logic

For each backup snapshot, the system asks: "Should this be kept?"

1. **If backup is ≤ 7 days old**: KEEP (daily backup)
   - Example: Backup from 3 days ago → KEEP
   - Reason: Latest backups have highest value

2. **If backup is 8-28 days old AND it's a Sunday**: KEEP (weekly backup)
   - Example: Backup from last Sunday (5 days ago at time of restore) → KEEP
   - Reason: Weekly cadence captures week-over-week changes

3. **If backup is 29-180 days old AND it's the 1st of month**: KEEP (monthly backup)
   - Example: Backup from January 1st → KEEP (if within 180 days)
   - Reason: Monthly retention for long-term compliance/archival

4. **Otherwise**: DELETE
   - Example: Backup from 35 days ago on a Wednesday → DELETE
   - Reason: Doesn't match any retention tier

### Retention Examples

Given today is **Friday, January 19, 2024**:

| Backup Date | Age | Keep? | Reason |
|---|---|---|---|
| Jan 19 (today) | 0d | ✅ YES | Daily |
| Jan 18 | 1d | ✅ YES | Daily |
| Jan 15 (Mon) | 4d | ✅ YES | Daily |
| Jan 14 (Sun) | 5d | ✅ YES | Daily (also Sunday) |
| Jan 10 (Wed) | 9d | ❌ NO | Outside daily window, not Sunday |
| Jan 7 (Sun) | 12d | ✅ YES | Weekly (Sunday) |
| Jan 1 (Mon) | 18d | ✅ YES | Monthly (1st of month) |
| Dec 25 | 25d | ❌ NO | Not 1st of month, not in 28d window |
| Oct 1 | 110d | ✅ YES | Monthly (1st of month, within 180d) |
| July 1 | 202d | ❌ NO | Monthly but > 180 days old |

### Storage Cost Estimate

Based on typical PostgreSQL database sizes:

| Environment | Daily Backups | Weekly Backups | Monthly Backups | Est. Storage | Est. Monthly Cost |
|---|---|---|---|---|---|
| dev | 7 × 10GB | 4 × 10GB | 6 × 10GB | ~210 GB | ~$5 |
| staging | 7 × 50GB | 4 × 50GB | 6 × 50GB | ~1.05 TB | ~$25 |
| production | 7 × 100GB | 4 × 100GB | 6 × 100GB | ~2.1 TB | ~$50 |

(Assumes S3 INTELLIGENT_TIERING: $0.023/GB for first 90 days, $0.0125/GB after)

## Backup Operations

### Manual Backup

To create an explicit backup outside the scheduled window:

```bash
# Using GitHub Actions workflow (recommended)
gh workflow run backup-database.yml \
  -f environment=production \
  -f backup_type=manual

# Or via CLI script
./scripts/backup-database.sh production manual
```

### Verify Backup

To verify backup system is functioning:

```bash
# Using workflow (automatic daily check)
gh workflow run backup-database.yml \
  -f environment=production \
  -f backup_type=verify

# Or manually via CLI
./scripts/backup-database.sh production verify
```

Output includes:
- Recent RDS snapshots
- Snapshots in S3
- Backup retention status
- Last backup timestamp

### Monitor Backups

To monitor backup health:

```bash
# List recent snapshots
aws rds describe-db-snapshots \
  --db-instance-identifier stellar-insights-production \
  --query 'DBSnapshots[0:5].[DBSnapshotIdentifier, SnapshotCreateTime, Status]' \
  --output table

# Check export tasks
aws rds describe-export-tasks \
  --query 'ExportTasks[*].[ExportTaskIdentifier, Status, PercentProgress]' \
  --output table

# Monitor S3 backups
aws s3 ls s3://stellar-insights-backups/database/production/ --recursive --human-readable
```

## Database Restoration

### Scenario 1: Restore from Recent Loss (< 7 days)

**Symptom**: Data accidentally deleted or corrupted in the last few days.

**Recovery Time**: 5-10 minutes

**Procedure**:

1. **Identify the snapshot you want to restore from**:
   ```bash
   aws rds describe-db-snapshots \
     --db-instance-identifier stellar-insights-production \
     --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[-1:].{ID:DBSnapshotIdentifier, Created:SnapshotCreateTime}' \
     --output table
   ```

2. **Create a new database instance from snapshot**:
   ```bash
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-identifier stellar-insights-production-restored \
     --db-snapshot-identifier stellar-insights-production-backup-20240119-020001 \
     --db-instance-class db.t3.medium \
     --multi-az \
     --vpc-security-group-ids sg-xxxxxxxx \
     --db-subnet-group-name stellar-insights-db-production \
     --publicly-accessible false
   ```

3. **Wait for restoration to complete** (5-10 minutes):
   ```bash
   aws rds wait db-instance-available \
     --db-instance-identifier stellar-insights-production-restored
   ```

4. **Verify restored database**:
   ```bash
   # Check instance is healthy
   aws rds describe-db-instances \
     --db-instance-identifier stellar-insights-production-restored \
     --query 'DBInstances[0].[Endpoint.Address, DBInstanceStatus, AllocatedStorage]'

   # Connect and run queries
   psql -h <new-endpoint> -U postgres -d stellar_insights -c "SELECT COUNT(*) FROM users;"
   ```

5. **If restored data looks good**: Update application connection string to new endpoint

6. **If restored data is corrupt**: Restore to earlier snapshot and retry

7. **Cleanup old instance** (once verified):
   ```bash
   aws rds delete-db-instance \
     --db-instance-identifier stellar-insights-production \
     --create-final-snapshot \
     --final-db-snapshot-identifier stellar-insights-production-backup-final-20240119

   # Rename restored instance to original name
   # (AWS doesn't support in-place rename, so manual DNS update needed)
   ```

### Scenario 2: Restore from Week-Old Backup (8-28 days)

**Symptom**: Major data corruption discovered a week later.

**Recovery Time**: 10-15 minutes

**Procedure**:

1. **Find the previous Sunday's snapshot**:
   ```bash
   # Calculate last Sunday's date
   LAST_SUNDAY=$(date -d "last Sunday" +%Y%m%d)

   # Find matching snapshot
   aws rds describe-db-snapshots \
     --db-instance-identifier stellar-insights-production \
     --query "DBSnapshots[?contains(DBSnapshotIdentifier, '${LAST_SUNDAY}')].{ID:DBSnapshotIdentifier, Age:CreateTime}" \
     --output table
   ```

2. **Follow Scenario 1 steps 2-7 using the weekly snapshot**

### Scenario 3: Restore from Monthly Archive (29-180 days)

**Symptom**: Compliance audit discovers need for historical data from 2 months ago.

**Recovery Time**: 20-30 minutes

**Procedure**:

1. **List monthly backups**:
   ```bash
   aws rds describe-db-snapshots \
     --db-instance-identifier stellar-insights-production \
     --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[].{ID:DBSnapshotIdentifier, Created:SnapshotCreateTime, Age:tags[?Key==`Type`].Value}' \
     --output table
   ```

2. **Choose 1st-of-month snapshot** (e.g., Feb 1 for February data)

3. **Follow Scenario 1 steps 2-7**

### Scenario 4: Disaster Recovery (Instance Lost)

**Symptom**: Database instance deleted or unrecoverable.

**Recovery Time**: 15-20 minutes

**Procedure**:

1. **Verify deletion**:
   ```bash
   aws rds describe-db-instances \
     --db-instance-identifier stellar-insights-production \
     --query 'DBInstances[0].DBInstanceStatus'
   ```

2. **Find latest good snapshot**:
   ```bash
   aws rds describe-db-snapshots \
     --db-instance-identifier stellar-insights-production \
     --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[-1:].DBSnapshotIdentifier' \
     --output text
   ```

3. **Restore with original configuration**:
   ```bash
   # Use original instance ID and configuration
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-identifier stellar-insights-production \
     --db-snapshot-identifier <latest-snapshot> \
     --db-instance-class db.t3.medium \
     --multi-az \
     --vpc-security-group-ids sg-xxxxxxxx \
     --db-subnet-group-name stellar-insights-db-production \
     --enable-cloudwatch-logs-exports postgresql \
     --enable-iam-database-authentication
   ```

4. **Restore will also restore**:
   - Database parameters (character set, timezone, etc.)
   - Security groups (but verify)
   - Backup retention settings
   - Multi-AZ configuration
   - Parameter group settings

5. **Verify and switch application traffic** once healthy

### Important: Testing Restore Procedures

**Monthly Restore Drill**: On the first Monday of each month, simulate a restore:

```bash
#!/bin/bash
# Monthly restore test
SNAPSHOT=$(aws rds describe-db-snapshots \
  --db-instance-identifier stellar-insights-production \
  --query 'sort_by(DBSnapshots, &SnapshotCreateTime)[-1].DBSnapshotIdentifier' \
  --output text)

echo "Testing restore from: $SNAPSHOT"

# Create test instance
TEST_ID="stellar-insights-production-restore-test-$(date +%s)"
aws rds restore-db-instance-from-db-snapshot \
  --db-instance-identifier "$TEST_ID" \
  --db-snapshot-identifier "$SNAPSHOT" \
  --db-instance-class db.t3.micro  # Use smaller instance for testing

# Wait for completion
aws rds wait db-instance-available --db-instance-identifier "$TEST_ID"

# Verify data integrity
psql -h $(aws rds describe-db-instances --db-instance-identifier "$TEST_ID" --query 'DBInstances[0].Endpoint.Address' --output text) \
  -U postgres -d stellar_insights -c "SELECT COUNT(*) FROM users; SELECT COUNT(*) FROM analytics;"

# Cleanup
aws rds delete-db-instance --db-instance-identifier "$TEST_ID" --skip-final-snapshot

echo "✓ Restore test completed successfully"
```

Schedule this with:
```bash
# Add to crontab
0 9 1 * * /path/to/monthly-restore-test.sh >> /var/log/backup-test.log 2>&1
```

## Integration with Terraform

The backup system integrates with Terraform infrastructure:

### RDS Configuration

```hcl
module "database" {
  source = "../../modules/database"

  # Backup configuration
  backup_retention_period  = 30  # production: 30 days
  backup_window           = "02:00-03:00"  # UTC
  preferred_maintenance_window = "sun:03:00-sun:04:00"  # After backup

  # Enable automated backups
  skip_final_snapshot = false
  final_snapshot_identifier = "stellar-insights-production-final-$(date +%s)"

  # Encryption & monitoring
  storage_encrypted = true
  kms_key_id       = aws_kms_key.rds.arn
  enable_enhanced_monitoring = true

  # Multi-AZ failover
  multi_az = true
}
```

### S3 Bucket Configuration

```hcl
resource "aws_s3_bucket" "backups" {
  bucket = "stellar-insights-backups"

  lifecycle {
    prevent_destroy = true  # Protect from accidental deletion
  }
}

# Backup retention via S3 lifecycle policy
resource "aws_s3_bucket_lifecycle_configuration" "backups" {
  bucket = aws_s3_bucket.backups.id

  rule {
    id     = "delete-old-backups"
    status = "Enabled"

    # Transition to Glacier after 90 days (cheaper storage)
    transition {
      days          = 90
      storage_class = "GLACIER"
    }

    # Delete after 180 days
    expiration {
      days = 180
    }

    filter {
      prefix = "database/"
    }
  }
}
```

## Troubleshooting

### Backup Failed to Complete

**Symptom**: Backup job failed or timed out in GitHub Actions

**Diagnosis**:
1. Check GitHub Actions logs: https://github.com/Ndifreke000/stellar-insights/actions/workflows/backup-database.yml
2. Verify RDS instance is healthy:
   ```bash
   aws rds describe-db-instances --db-instance-identifier stellar-insights-production
   ```
3. Check RDS event logs:
   ```bash
   aws rds describe-events --source-type db-instance --source-identifier stellar-insights-production
   ```

**Solutions**:
- If instance is unhealthy, restore from latest snapshot
- If quota exceeded, delete old snapshots manually
- If IAM permissions missing, verify role in AWS IAM console

### Restore Takes Too Long

**Symptom**: Restore is taking 30+ minutes

**Cause**: Large database size or storage I/O bottleneck

**Solution**:
1. Upgrade temporary instance to larger class during restore:
   ```bash
   # Use db.t3.large for faster restore
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-class db.t3.large \
     ...
   ```
2. Restore to a different AZ to avoid I/O contention
3. After restore completes, downgrade back to normal instance class

### Snapshot Import Fails

**Symptom**: S3 export task fails

**Cause**: Missing IAM role or S3 permissions

**Solution**:
1. Verify `rds-s3-export-role` exists in IAM
2. Add S3 bucket permissions to role:
   ```json
   {
     "Effect": "Allow",
     "Action": [
       "s3:PutObject*",
       "s3:GetObject*",
       "s3:DeleteObject*"
     ],
     "Resource": "arn:aws:s3:::stellar-insights-backups/*"
   }
   ```

## Related Documentation

- [Terraform Infrastructure](../terraform/README.md) - Infrastructure provisioning
- [Disaster Recovery Plan](disaster-recovery.md) - Full DR runbooks
- [CI/CD Optimization](CI_CD_OPTIMIZATION.md) - Backup automation pipeline
- [AWS RDS Backup](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_BackupRestore.html)
- [S3 Lifecycle Policies](https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-lifecycle-mgmt.html)

## Support & Escalation

For backup-related issues:

1. **Critical**: Database is inaccessible
   - Escalate to on-call DBA
   - Trigger manual restore from latest snapshot
   - See Disaster Recovery Plan

2. **High**: Backup job failing for 24+ hours
   - Escalate to DevOps team
   - Check all prerequisites (IAM, S3, RDS)
   - Run manual backup as interim measure

3. **Medium**: Backup is slow but completing
   - Monitor but no immediate action needed
   - Monitor disk space in S3
   - Consider upgrading database instance

4. **Low**: Need to verify backup functionality
   - Run monthly restore test
   - Check retention policy logic
   - Review past week of backup logs

## Change Log

| Date | Change | Reason |
|---|---|---|
| 2024-01 | Created backup system | Issue #2146 |
