#!/bin/bash

# Load environment variables
source ../../.env

# Check if wal-g is installed
if ! command -v wal-g &> /dev/null; then
    echo "wal-g is not installed. Please install wal-g."
    exit 1
fi

# Perform full backup
echo "Starting full backup..."
if wal-g backup-push $PGDATA; then
    echo "Full backup completed successfully."

    # Retention policy
    echo "Applying retention policy..."
    wal-g delete retain FULL $BACKUP_RETENTION_DAYS --confirm

    # Run restore test
    echo "Running restore test..."
    ./restore_test.sh

    # Notification
    echo "Backup successful" | mail -s "Database Backup Success" $NOTIFICATION_EMAIL
else
    echo "Full backup failed."
    echo "Backup failed" | mail -s "Database Backup Failure" $NOTIFICATION_EMAIL
    exit 1
fi