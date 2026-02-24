# Backup and Restore Procedures

**Last Updated:** 2024  
**Document Owner:** DevOps Team

---

## Backup Strategy

### Backup Types

| Type | Frequency | Retention | Storage | RPO |
|------|-----------|-----------|---------|-----|
| Database Full | Daily (2 AM) | 30 days | S3 + Local | 24 hours |
| Database Incremental (WAL) | Continuous | 30 days | S3 | 15 minutes |
| Configuration | On change | 90 days | Git + S3 | 1 hour |
| Application Logs | Hourly | 30 days | S3 | 1 hour |
| System Snapshots | Weekly | 4 weeks | Cloud Provider | 7 days |

### Backup Locations

**Primary:** AWS S3 `s3://stellar-insights-backups/`
```
stellar-insights-backups/
├── postgres/
│   ├── full/
│   └── wal/
├── redis/
├── configs/
└── logs/
```

**Secondary:** Local `/var/backups/`

---

## Database Backup

### Automated Backup (wal-g)

**Configuration:**
```bash
# /etc/postgresql/14/main/postgresql.conf
archive_mode = on
archive_command = 'wal-g wal-push %p'
archive_timeout = 60
```

**Environment:**
```bash
# /etc/environment
export WALG_S3_PREFIX=s3://stellar-insights-backups/postgres
export AWS_REGION=us-east-1
export PGDATA=/var/lib/postgresql/14/main
```

### Manual Backup

```bash
# Full backup
sudo -u postgres wal-g backup-push $PGDATA

# Verify backup
wal-g backup-list

# Test restore (to test environment)
wal-g backup-fetch /tmp/test-restore LATEST
```

### Backup Verification

```bash
#!/bin/bash
# /opt/scripts/verify-backup.sh

# List recent backups
BACKUPS=$(wal-g backup-list | tail -5)

if [ -z "$BACKUPS" ]; then
    echo "ERROR: No backups found"
    exit 1
fi

# Check backup age
LATEST=$(wal-g backup-list | tail -1 | awk '{print $2}')
AGE=$(( $(date +%s) - $(date -d "$LATEST" +%s) ))

if [ $AGE -gt 86400 ]; then
    echo "WARNING: Latest backup is older than 24 hours"
    exit 1
fi

echo "Backup verification passed"
```

---

## Database Restore

### Point-in-Time Recovery

```bash
# 1. Stop PostgreSQL
sudo systemctl stop postgresql

# 2. Backup current data
sudo mv $PGDATA ${PGDATA}.old

# 3. Restore base backup
sudo -u postgres wal-g backup-fetch $PGDATA LATEST

# 4. Configure recovery
sudo -u postgres cat > $PGDATA/recovery.conf <<EOF
restore_command = 'wal-g wal-fetch %f %p'
recovery_target_time = '2024-01-15 14:30:00'
recovery_target_action = 'promote'
EOF

# 5. Start PostgreSQL
sudo systemctl start postgresql

# 6. Monitor recovery
tail -f /var/log/postgresql/postgresql-14-main.log

# 7. Verify recovery
psql -U postgres -c "SELECT pg_is_in_recovery();"
```

### Restore to Specific Backup

```bash
# List backups
wal-g backup-list

# Restore specific backup
sudo -u postgres wal-g backup-fetch $PGDATA backup_20240115T020000Z

# Start recovery
sudo systemctl start postgresql
```

---

## Configuration Backup

### Automated Backup

```bash
#!/bin/bash
# /opt/scripts/backup-configs.sh

BACKUP_DIR="/var/backups/configs"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup directory
mkdir -p $BACKUP_DIR

# Backup application configs
tar -czf $BACKUP_DIR/app-configs-$DATE.tar.gz \
  /opt/stellar-insights/backend/.env \
  /opt/stellar-insights/frontend/.env.local \
  /etc/nginx/sites-available/stellar-insights

# Backup database configs
tar -czf $BACKUP_DIR/db-configs-$DATE.tar.gz \
  /etc/postgresql/14/main/postgresql.conf \
  /etc/postgresql/14/main/pg_hba.conf

# Upload to S3
aws s3 cp $BACKUP_DIR/ s3://stellar-insights-backups/configs/ --recursive

# Clean old backups (keep 90 days)
find $BACKUP_DIR -name "*.tar.gz" -mtime +90 -delete
```

### Configuration Restore

```bash
# Download from S3
aws s3 cp s3://stellar-insights-backups/configs/app-configs-latest.tar.gz /tmp/

# Extract
tar -xzf /tmp/app-configs-latest.tar.gz -C /

# Restart services
sudo systemctl restart stellar-insights-backend
sudo systemctl restart nginx
```

---

## Redis Backup

### Automated Backup

```bash
# Redis configuration
# /etc/redis/redis.conf
save 900 1
save 300 10
save 60 10000
dir /var/lib/redis
dbfilename dump.rdb
```

### Manual Backup

```bash
# Trigger save
redis-cli BGSAVE

# Copy RDB file
cp /var/lib/redis/dump.rdb /var/backups/redis/dump-$(date +%Y%m%d).rdb

# Upload to S3
aws s3 cp /var/backups/redis/dump-$(date +%Y%m%d).rdb s3://stellar-insights-backups/redis/
```

### Redis Restore

```bash
# Stop Redis
sudo systemctl stop redis

# Restore RDB file
sudo cp /var/backups/redis/dump-20240115.rdb /var/lib/redis/dump.rdb
sudo chown redis:redis /var/lib/redis/dump.rdb

# Start Redis
sudo systemctl start redis

# Verify
redis-cli PING
```

---

## Backup Monitoring

### Backup Health Checks

```bash
#!/bin/bash
# /opt/scripts/check-backup-health.sh

# Check database backups
DB_BACKUP_AGE=$(wal-g backup-list | tail -1 | awk '{print $2}')
DB_AGE_SECONDS=$(( $(date +%s) - $(date -d "$DB_BACKUP_AGE" +%s) ))

if [ $DB_AGE_SECONDS -gt 86400 ]; then
    echo "CRITICAL: Database backup older than 24 hours"
    exit 2
fi

# Check S3 connectivity
aws s3 ls s3://stellar-insights-backups/ > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "CRITICAL: Cannot access S3 backup bucket"
    exit 2
fi

# Check disk space
DISK_USAGE=$(df -h /var/backups | tail -1 | awk '{print $5}' | sed 's/%//')
if [ $DISK_USAGE -gt 80 ]; then
    echo "WARNING: Backup disk usage above 80%"
    exit 1
fi

echo "OK: All backup checks passed"
exit 0
```

### Backup Alerts

Configure monitoring to alert on:
- Backup age > 24 hours
- Backup failures
- S3 access issues
- Disk space > 80%
- WAL archive lag > 15 minutes

---

## Backup Testing

### Weekly Restore Test

```bash
#!/bin/bash
# /opt/scripts/test-restore.sh

TEST_DIR="/tmp/restore-test-$(date +%Y%m%d)"
mkdir -p $TEST_DIR

# Restore latest backup
wal-g backup-fetch $TEST_DIR LATEST

# Start test instance
docker run -d \
  --name postgres-restore-test \
  -v $TEST_DIR:/var/lib/postgresql/data \
  -e POSTGRES_PASSWORD=test \
  postgres:14

# Wait for startup
sleep 10

# Test connectivity
docker exec postgres-restore-test psql -U postgres -c "SELECT version();"

# Verify data
docker exec postgres-restore-test psql -U postgres -d stellar_insights -c "
  SELECT COUNT(*) FROM anchors;
  SELECT COUNT(*) FROM corridors;
"

# Cleanup
docker stop postgres-restore-test
docker rm postgres-restore-test
rm -rf $TEST_DIR

echo "Restore test completed successfully"
```

---

## Retention Policy

### Automated Cleanup

```bash
#!/bin/bash
# /opt/scripts/cleanup-old-backups.sh

# Delete backups older than retention period
wal-g delete retain FULL 30 --confirm

# Clean local backups
find /var/backups/postgres -name "*.backup" -mtime +30 -delete
find /var/backups/configs -name "*.tar.gz" -mtime +90 -delete
find /var/backups/redis -name "dump-*.rdb" -mtime +30 -delete

# Clean S3 backups
aws s3 ls s3://stellar-insights-backups/postgres/full/ | \
  awk '{print $4}' | \
  while read backup; do
    AGE=$(( $(date +%s) - $(date -d "$(echo $backup | cut -d_ -f2)" +%s) ))
    if [ $AGE -gt 2592000 ]; then  # 30 days
      aws s3 rm s3://stellar-insights-backups/postgres/full/$backup
    fi
  done
```

---

## Disaster Recovery Testing

### Quarterly DR Drill

**Objectives:**
1. Verify backup integrity
2. Test restore procedures
3. Measure recovery time
4. Validate documentation

**Procedure:**
1. Schedule maintenance window
2. Take final backup
3. Simulate disaster (delete data)
4. Execute restore procedures
5. Verify data integrity
6. Measure time to recovery
7. Document results
8. Update procedures

---

## Backup Checklist

### Daily
- [ ] Verify automated backups completed
- [ ] Check backup logs for errors
- [ ] Monitor backup storage usage

### Weekly
- [ ] Test restore to staging environment
- [ ] Verify backup retention policy
- [ ] Review backup performance metrics

### Monthly
- [ ] Full restore test
- [ ] Review and update backup procedures
- [ ] Audit backup access logs

### Quarterly
- [ ] Complete DR drill
- [ ] Review backup strategy
- [ ] Update documentation

---

## Troubleshooting

### Backup Failures

```bash
# Check wal-g logs
journalctl -u postgresql | grep wal-g

# Check S3 permissions
aws s3 ls s3://stellar-insights-backups/

# Check disk space
df -h /var/lib/postgresql

# Manual backup
sudo -u postgres wal-g backup-push $PGDATA
```

### Restore Failures

```bash
# Check recovery logs
tail -f /var/log/postgresql/postgresql-14-main.log

# Verify backup exists
wal-g backup-list

# Check permissions
ls -la $PGDATA

# Check recovery.conf
cat $PGDATA/recovery.conf
```

---

**Last Tested:** [Date]  
**Next Review:** [Date]  
**Contact:** devops@stellar-insights.com
