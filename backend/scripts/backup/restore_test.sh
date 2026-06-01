#!/bin/bash

# Load environment variables
source ../../.env

# Check if wal-g is installed
if ! command -v wal-g &> /dev/null; then
    echo "wal-g is not installed. Please install wal-g."
    exit 1
fi

# Test restore by fetching latest backup to temp directory
TEMP_DIR=/tmp/test_restore_$(date +%s)
mkdir -p $TEMP_DIR

echo "Testing restore by fetching latest backup to $TEMP_DIR..."
if wal-g backup-fetch $TEMP_DIR LATEST; then
    echo "Restore test successful: Backup fetched successfully."

    # Clean up
    rm -rf $TEMP_DIR

    # Notification
    echo "Restore test successful" | mail -s "Database Restore Test Success" $NOTIFICATION_EMAIL
else
    echo "Restore test failed."
    rm -rf $TEMP_DIR
    echo "Restore test failed" | mail -s "Database Restore Test Failure" $NOTIFICATION_EMAIL
    exit 1
fi