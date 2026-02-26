# Database Backup Setup

This directory contains scripts for automated PostgreSQL backups using wal-g.

## Prerequisites

- PostgreSQL installed and running
- wal-g installed (https://github.com/wal-g/wal-g)
- AWS CLI configured with access to S3 bucket
- mail command configured for notifications (sendmail or similar)

## Configuration

1. Copy `backend/.env.example` to `backend/.env` and fill in the values:
   - `DATABASE_URL`: PostgreSQL connection string
   - `BACKUP_S3_BUCKET`: Your S3 bucket name
   - `NOTIFICATION_EMAIL`: Email for notifications
   - `PGDATA`: Path to PostgreSQL data directory (usually /var/lib/postgresql/data)

2. Set AWS credentials as environment variables or in ~/.aws/credentials:
   - AWS_ACCESS_KEY_ID
   - AWS_SECRET_ACCESS_KEY
   - AWS_REGION

## PostgreSQL Configuration

Add the following to postgresql.conf:

```
wal_level = replica
archive_mode = on
archive_command = 'wal-g wal-push %p'
archive_timeout = 60
```

Restart PostgreSQL after changes.

## Setup

1. Run `./setup_cron.sh` to schedule backups (requires sudo).

## Scripts

- `full_backup.sh`: Daily full backup
- `incremental_backup.sh`: Hourly incremental backup
- `restore_test.sh`: Automated restore testing
- `setup_cron.sh`: Set up cron jobs

## Backup Schedule

- Full backup: Daily at 2 AM
- Incremental backup: Every hour
- Retention: 30 days
- Restore test: After each full backup