#!/bin/bash

# Load environment variables
source ../../.env

# Check if wal-g is installed
if ! command -v wal-g &> /dev/null; then
    echo "wal-g is not installed. Please install wal-g."
    exit 1
fi

# Perform incremental backup
echo "Starting incremental backup..."
if wal-g backup-push --delta $PGDATA; then
    echo "Incremental backup completed successfully."

    # Notification
    echo "Incremental backup successful" | mail -s "Database Incremental Backup Success" $NOTIFICATION_EMAIL
else
    echo "Incremental backup failed."
    echo "Incremental backup failed" | mail -s "Database Incremental Backup Failure" $NOTIFICATION_EMAIL
    exit 1
fi