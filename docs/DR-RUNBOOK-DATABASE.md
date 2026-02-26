# Database Disaster Recovery Runbook

**Service:** PostgreSQL Database  
**Priority:** P0 - Critical  
**RTO:** 1 hour  
**RPO:** 15 minutes  
**Last Updated:** 2024

---

## Quick Reference

| Scenario | Procedure | Time Estimate |
|----------|-----------|---------------|
| Database Crash | [Restart Database](#scenario-1-database-crash) | 10 minutes |
| Data Corruption | [Point-in-Time Recovery](#scenario-2-data-corruption) | 45 minutes |
| Complete Loss | [Full Restore](#scenario-3-complete-database-loss) | 1 hour |
| Replication Failure | [Rebuild Replica](#scenario-4-replication-failure) | 30 minutes |

---

## Prerequisites

### Required Access
- [ ] SSH access to database server
- [ ] PostgreSQL superuser credentials
- [ ] AWS S3 access (for backups)
- [ ] Monitoring dashboard access
- [ ] PagerDuty/alerting system access

### Required Tools
```bash
# Verify tools are installed
psql --version          # PostgreSQL client
wal-g --version         # Backup/restore tool
aws --version           # AWS CLI
pg_isready             # Connection checker
```

### Backup Locations
- **Primary:** `s3://stellar-insights-backups/postgres/`
- **Secondary:** `/var/backups/postgresql/`
- **Retention:** 30 days full, 90 days incremental

---

## Scenario 1: Database Crash

**Symptoms:**
- Database not accepting connections
- Application errors: "connection refused"
- Monitoring alerts: "PostgreSQL down"

### Detection

```bash
# Check if PostgreSQL is running
sudo systemctl status postgresql

# Check database connectivity
pg_isready -h localhost -p 5432

# Check logs
sudo tail -f /var/log/postgresql/postgresql-14-main.log
```

### Recovery Steps

#### Step 1: Assess the Situation (5 minutes)

```bash
# Check system resources
free -h                 # Memory
df -h                   # Disk space
top                     # CPU usage

# Check for OOM killer
dmesg | grep -i "out of memory"

# Check PostgreSQL logs
sudo tail -100 /var/log/postgresql/postgresql-14-main.log
```

**Decision Point:**
- If disk full → Free up space first
- If OOM killed → Increase memory or tune config
- If corruption suspected → Go to Scenario 2
- Otherwise → Proceed to restart

#### Step 2: Attempt Restart (5 minutes)

```bash
# Stop PostgreSQL gracefully
sudo systemctl stop postgresql

# Wait for clean shutdown
sleep 10

# Start PostgreSQL
sudo systemctl start postgresql

# Verify status
sudo systemctl status postgresql

# Test connectivity
psql -h localhost -U postgres -c "SELECT version();"
```

**Validation:**
```bash
# Check database is accepting connections
pg_isready -h localhost -p 5432

# Verify replication (if applicable)
psql -U postgres -c "SELECT * FROM pg_stat_replication;"

# Check for errors
sudo tail -20 /var/log/postgresql/postgresql-14-main.log
```

#### Step 3: Verify Application Connectivity

```bash
# Test from application server
curl http://localhost:8080/health

# Check backend logs
docker logs stellar-insights-backend --tail 50

# Monitor error rates
# Check monitoring dashboard for error spike
```

### Rollback Plan

If restart fails:
1. Check for lock files: `sudo rm /var/run/postgresql/.s.PGSQL.5432.lock`
2. Try single-user mode: `postgres --single -D /var/lib/postgresql/14/main`
3. If still failing → Escalate to Scenario 3 (Full Restore)

---

## Scenario 2: Data Corruption

**Symptoms:**
- Database starts but queries fail
- Checksum errors in logs
- Index corruption errors
- Inconsistent query results

### Detection

```bash
# Check for corruption in logs
sudo grep -i "corruption\|checksum\|invalid" /var/log/postgresql/postgresql-14-main.log

# Run integrity checks
psql -U postgres -d stellar_insights -c "
  SELECT datname, pg_database_size(datname) 
  FROM pg_database 
  WHERE datname = 'stellar_insights';
"

# Check table integrity
psql -U postgres -d stellar_insights -c "
  SELECT schemaname, tablename, pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename))
  FROM pg_tables 
  WHERE schemaname = 'public'
  ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;
"
```

### Recovery Steps

#### Step 1: Identify Corruption Scope (10 minutes)

```bash
# Identify affected tables
psql -U postgres -d stellar_insights <<EOF
-- Check for corrupted indexes
SELECT schemaname, tablename, indexname
FROM pg_indexes
WHERE schemaname = 'public';

-- Try to access each table
\dt
EOF

# Document affected tables
echo "Affected tables:" > /tmp/corruption_report.txt
```

#### Step 2: Attempt Index Rebuild (15 minutes)

```bash
# If only indexes are corrupted
psql -U postgres -d stellar_insights <<EOF
-- Reindex all tables
REINDEX DATABASE stellar_insights;

-- Verify
SELECT schemaname, tablename, indexname
FROM pg_indexes
WHERE schemaname = 'public';
EOF
```

**Validation:**
```bash
# Test queries
psql -U postgres -d stellar_insights -c "
  SELECT COUNT(*) FROM anchors;
  SELECT COUNT(*) FROM corridors;
  SELECT COUNT(*) FROM metrics;
"
```

#### Step 3: Point-in-Time Recovery (30 minutes)

If reindex fails, restore from backup:

```bash
# 1. Stop application to prevent new writes
sudo systemctl stop stellar-insights-backend

# 2. Stop PostgreSQL
sudo systemctl stop postgresql

# 3. Backup current (corrupted) data
sudo mv /var/lib/postgresql/14/main /var/lib/postgresql/14/main.corrupted.$(date +%Y%m%d_%H%M%S)

# 4. Restore from backup
export PGDATA=/var/lib/postgresql/14/main
export AWS_REGION=us-east-1

# List available backups
wal-g backup-list

# Restore latest backup
wal-g backup-fetch $PGDATA LATEST

# 5. Configure recovery
cat > /var/lib/postgresql/14/main/recovery.conf <<EOF
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_time = '$(date -u -d '15 minutes ago' '+%Y-%m-%d %H:%M:%S')'
recovery_target_action = 'promote'
EOF

# 6. Start PostgreSQL in recovery mode
sudo systemctl start postgresql

# 7. Monitor recovery
tail -f /var/log/postgresql/postgresql-14-main.log
```

**Validation:**
```bash
# Check recovery status
psql -U postgres -c "SELECT pg_is_in_recovery();"

# Verify data integrity
psql -U postgres -d stellar_insights -c "
  SELECT 
    COUNT(*) as total_anchors,
    MAX(updated_at) as last_update
  FROM anchors;
"

# Check for missing data
psql -U postgres -d stellar_insights -c "
  SELECT 
    DATE(created_at) as date,
    COUNT(*) as records
  FROM metrics
  GROUP BY DATE(created_at)
  ORDER BY date DESC
  LIMIT 7;
"
```

#### Step 4: Restart Application

```bash
# Start backend service
sudo systemctl start stellar-insights-backend

# Verify health
curl http://localhost:8080/health

# Check logs
docker logs stellar-insights-backend --tail 50
```

### Rollback Plan

If PITR fails:
1. Stop PostgreSQL
2. Restore from previous day's full backup
3. Accept data loss up to last full backup
4. Document data loss period
5. Notify stakeholders

---

## Scenario 3: Complete Database Loss

**Symptoms:**
- Database server unreachable
- Data directory missing/corrupted
- Hardware failure
- Ransomware attack

### Recovery Steps

#### Step 1: Provision New Database Server (15 minutes)

```bash
# If using cloud infrastructure
# Launch new EC2 instance or RDS instance

# Install PostgreSQL
sudo apt-get update
sudo apt-get install -y postgresql-14 postgresql-contrib-14

# Install wal-g
wget https://github.com/wal-g/wal-g/releases/download/v2.0.1/wal-g-pg-ubuntu-20.04-amd64.tar.gz
sudo tar -xzf wal-g-pg-ubuntu-20.04-amd64.tar.gz -C /usr/local/bin/
sudo chmod +x /usr/local/bin/wal-g
```

#### Step 2: Configure PostgreSQL (10 minutes)

```bash
# Stop default PostgreSQL
sudo systemctl stop postgresql

# Configure PostgreSQL
sudo -u postgres cat > /etc/postgresql/14/main/postgresql.conf <<EOF
# Connection Settings
listen_addresses = '*'
port = 5432
max_connections = 100

# Memory Settings
shared_buffers = 256MB
effective_cache_size = 1GB
work_mem = 16MB

# WAL Settings
wal_level = replica
max_wal_senders = 3
wal_keep_size = 1GB

# Logging
logging_collector = on
log_directory = '/var/log/postgresql'
log_filename = 'postgresql-%Y-%m-%d_%H%M%S.log'
log_line_prefix = '%t [%p]: [%l-1] user=%u,db=%d,app=%a,client=%h '
EOF

# Configure authentication
sudo -u postgres cat > /etc/postgresql/14/main/pg_hba.conf <<EOF
# TYPE  DATABASE        USER            ADDRESS                 METHOD
local   all             postgres                                peer
local   all             all                                     peer
host    all             all             127.0.0.1/32            md5
host    all             all             ::1/128                 md5
host    all             all             10.0.0.0/8              md5
EOF
```

#### Step 3: Restore from Backup (30 minutes)

```bash
# Set environment variables
export PGDATA=/var/lib/postgresql/14/main
export AWS_ACCESS_KEY_ID=your_key
export AWS_SECRET_ACCESS_KEY=your_secret
export AWS_REGION=us-east-1
export WALG_S3_PREFIX=s3://stellar-insights-backups/postgres

# List available backups
wal-g backup-list

# Restore latest backup
sudo -u postgres wal-g backup-fetch $PGDATA LATEST

# Configure recovery
sudo -u postgres cat > $PGDATA/recovery.conf <<EOF
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_timeline = 'latest'
EOF

# Set permissions
sudo chown -R postgres:postgres $PGDATA
sudo chmod 700 $PGDATA

# Start PostgreSQL
sudo systemctl start postgresql

# Monitor recovery
sudo tail -f /var/log/postgresql/postgresql-14-main.log
```

#### Step 4: Verify and Promote (10 minutes)

```bash
# Check recovery status
psql -U postgres -c "SELECT pg_is_in_recovery();"

# When recovery complete, promote to primary
psql -U postgres -c "SELECT pg_promote();"

# Verify promotion
psql -U postgres -c "SELECT pg_is_in_recovery();"  # Should return false

# Check database size
psql -U postgres -c "
  SELECT 
    pg_database.datname,
    pg_size_pretty(pg_database_size(pg_database.datname)) AS size
  FROM pg_database
  ORDER BY pg_database_size(pg_database.datname) DESC;
"
```

#### Step 5: Update Application Configuration

```bash
# Update backend .env file
cat >> /opt/stellar-insights/backend/.env <<EOF
DATABASE_URL=postgresql://postgres:password@new-db-host:5432/stellar_insights
EOF

# Restart backend
sudo systemctl restart stellar-insights-backend

# Verify connectivity
curl http://localhost:8080/health
```

#### Step 6: Resume Backups

```bash
# Configure WAL archiving
sudo -u postgres cat >> /etc/postgresql/14/main/postgresql.conf <<EOF
archive_mode = on
archive_command = 'wal-g wal-push %p'
EOF

# Reload configuration
sudo systemctl reload postgresql

# Take new full backup
sudo -u postgres wal-g backup-push $PGDATA

# Verify backup
wal-g backup-list
```

### Validation Checklist

- [ ] Database is running and accepting connections
- [ ] All tables are accessible
- [ ] Data integrity checks pass
- [ ] Application can connect and query
- [ ] Replication is working (if applicable)
- [ ] Backups are resuming
- [ ] Monitoring is active
- [ ] Performance is acceptable
- [ ] No errors in logs

---

## Scenario 4: Replication Failure

**Symptoms:**
- Replica lag increasing
- Replication slot inactive
- WAL files accumulating

### Recovery Steps

#### Step 1: Diagnose Replication Issue

```bash
# Check replication status on primary
psql -U postgres -c "SELECT * FROM pg_stat_replication;"

# Check replication lag
psql -U postgres -c "
  SELECT 
    client_addr,
    state,
    sent_lsn,
    write_lsn,
    flush_lsn,
    replay_lsn,
    sync_state,
    pg_wal_lsn_diff(sent_lsn, replay_lsn) AS lag_bytes
  FROM pg_stat_replication;
"

# Check WAL files
ls -lh /var/lib/postgresql/14/main/pg_wal/
```

#### Step 2: Restart Replication

```bash
# On replica server
sudo systemctl stop postgresql

# Remove old data directory
sudo rm -rf /var/lib/postgresql/14/main/*

# Create new base backup from primary
pg_basebackup -h primary-db-host -U replication -D /var/lib/postgresql/14/main -P -v

# Configure recovery
cat > /var/lib/postgresql/14/main/recovery.conf <<EOF
standby_mode = 'on'
primary_conninfo = 'host=primary-db-host port=5432 user=replication password=password'
restore_command = 'wal-g wal-fetch %f %p'
EOF

# Start replica
sudo systemctl start postgresql

# Verify replication
psql -U postgres -c "SELECT pg_is_in_recovery();"  # Should return true
```

---

## Common Issues and Solutions

### Issue: Out of Disk Space

**Solution:**
```bash
# Check disk usage
df -h

# Clean old WAL files (if archiving is working)
sudo -u postgres pg_archivecleanup /var/lib/postgresql/14/main/pg_wal/ $(ls -t /var/lib/postgresql/14/main/pg_wal/ | tail -1)

# Clean old log files
sudo find /var/log/postgresql/ -name "*.log" -mtime +7 -delete

# Vacuum database
psql -U postgres -d stellar_insights -c "VACUUM FULL VERBOSE;"
```

### Issue: Connection Pool Exhausted

**Solution:**
```bash
# Check active connections
psql -U postgres -c "
  SELECT 
    datname,
    count(*) as connections
  FROM pg_stat_activity
  GROUP BY datname;
"

# Kill idle connections
psql -U postgres -c "
  SELECT pg_terminate_backend(pid)
  FROM pg_stat_activity
  WHERE state = 'idle'
  AND state_change < NOW() - INTERVAL '10 minutes';
"

# Increase max_connections (requires restart)
sudo -u postgres psql -c "ALTER SYSTEM SET max_connections = 200;"
sudo systemctl restart postgresql
```

### Issue: Slow Queries

**Solution:**
```bash
# Identify slow queries
psql -U postgres -c "
  SELECT 
    pid,
    now() - pg_stat_activity.query_start AS duration,
    query,
    state
  FROM pg_stat_activity
  WHERE state != 'idle'
  ORDER BY duration DESC
  LIMIT 10;
"

# Analyze and vacuum
psql -U postgres -d stellar_insights -c "ANALYZE VERBOSE;"
psql -U postgres -d stellar_insights -c "VACUUM ANALYZE;"
```

---

## Post-Recovery Checklist

- [ ] Database is running normally
- [ ] All services connected
- [ ] Replication working (if applicable)
- [ ] Backups resuming
- [ ] Monitoring active
- [ ] Performance metrics normal
- [ ] No errors in logs
- [ ] Incident documented
- [ ] Post-mortem scheduled

---

## Escalation

If recovery is not progressing:
- **15 minutes:** Notify Technical Lead
- **30 minutes:** Notify Incident Commander
- **45 minutes:** Engage database vendor support
- **1 hour:** Escalate to CTO

---

## Related Documents

- [DISASTER_RECOVERY_PLAN.md](./DISASTER_RECOVERY_PLAN.md)
- [BACKUP-RESTORE-PROCEDURES.md](./BACKUP-RESTORE-PROCEDURES.md)
- [MONITORING-ALERTING.md](./MONITORING-ALERTING.md)

---

**Last Tested:** [Date]  
**Next Test:** [Date]  
**Document Owner:** DevOps Team
